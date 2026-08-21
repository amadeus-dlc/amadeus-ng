//! `CheckboxState` / Stage Progress 行 — 6 状態マーカー＋ em dash 区切り。パース文法は
//! `/^- \[([ xSR?-])\] (\S+)\s*—\s*(.*)$/gm` (upstream `aidlc-lib.ts:6678`, 03 §5.4)。
//! marker writer と suffix writer は同一行の互いに素なフィールドを編集する別 API
//! (recompose と jump が合成できる根拠)。suffix writer の正確な直列化は
//! TODO(golden: stage-0) — 本スライスは marker 側のみ実装する。

/// 6 状態 (01 §3.3 — E1)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckboxState {
    Pending,
    InProgress,
    AwaitingApproval,
    Revising,
    Completed,
    Skipped,
}

impl CheckboxState {
    #[must_use]
    pub const fn marker(self) -> char {
        match self {
            CheckboxState::Pending => ' ',
            CheckboxState::InProgress => '-',
            CheckboxState::AwaitingApproval => '?',
            CheckboxState::Revising => 'R',
            CheckboxState::Completed => 'x',
            CheckboxState::Skipped => 'S',
        }
    }

    #[must_use]
    pub const fn from_marker(c: char) -> Option<CheckboxState> {
        Some(match c {
            ' ' => CheckboxState::Pending,
            '-' => CheckboxState::InProgress,
            '?' => CheckboxState::AwaitingApproval,
            'R' => CheckboxState::Revising,
            'x' => CheckboxState::Completed,
            'S' => CheckboxState::Skipped,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckboxEntry {
    pub state: CheckboxState,
    pub slug: String,
    /// em dash 以降のテキスト (verbatim 保存 — title や EXECUTE/SKIP サフィックスを含む)。
    pub rest: String,
}

/// Stage Progress 行のパース。文法に一致しない行は黙って無視する (upstream 同等の寛容パース)。
#[must_use]
pub fn parse_checkboxes(content: &str) -> Vec<CheckboxEntry> {
    let mut out = Vec::new();
    for line in content.lines() {
        if let Some(entry) = parse_line(line) {
            out.push(entry);
        }
    }
    out
}

fn parse_line(line: &str) -> Option<CheckboxEntry> {
    // `- [<m>] <slug>\s*—\s*<rest>`
    let rest = line.strip_prefix("- [")?;
    let mut chars = rest.chars();
    let marker = chars.next()?;
    let state = CheckboxState::from_marker(marker)?;
    let rest = chars.as_str().strip_prefix("] ")?;
    let dash = rest.find('—')?;
    let (slug_part, tail) = rest.split_at(dash);
    let slug = slug_part.trim_end_matches([' ', '\t']);
    if slug.is_empty() || slug.contains(char::is_whitespace) {
        return None;
    }
    let tail = tail.strip_prefix('—').unwrap_or(tail);
    let tail = tail.trim_start_matches([' ', '\t']);
    Some(CheckboxEntry {
        state,
        slug: slug.to_string(),
        rest: tail.to_string(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckboxWriteError {
    /// 対象 slug の行が存在しない。
    MissingStage(String),
}

/// marker のみを書き換える (rest と suffix には触れない — 互いに素なフィールド編集)。
/// # Errors
///
/// 対象 slug の行が存在しなければ `MissingStage`。
pub fn set_checkbox(
    content: &str,
    slug: &str,
    state: CheckboxState,
) -> Result<String, CheckboxWriteError> {
    let mut found = false;
    let mut out: Vec<String> = Vec::new();
    for line in content.lines() {
        match parse_line(line) {
            Some(entry) if entry.slug == slug => {
                found = true;
                // 元の行の marker 1 文字だけを置換 (前後は verbatim 保存)
                let prefix_len = "- [".len();
                let mut rebuilt = String::with_capacity(line.len());
                rebuilt.push_str(&line[..prefix_len]);
                rebuilt.push(state.marker());
                let after_marker = line[prefix_len..]
                    .chars()
                    .next()
                    .map(char::len_utf8)
                    .unwrap_or(1);
                rebuilt.push_str(&line[prefix_len + after_marker..]);
                out.push(rebuilt);
            }
            _ => out.push(line.to_string()),
        }
    }
    if !found {
        return Err(CheckboxWriteError::MissingStage(slug.to_string()));
    }
    let mut result = out.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    Ok(result)
}

/// `Completed` フィールド同期のための集計 (upstream `countCheckboxes`)。
#[must_use]
pub fn count_completed(content: &str) -> usize {
    parse_checkboxes(content)
        .iter()
        .filter(|e| e.state == CheckboxState::Completed)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
## Stage Progress

### IDEATION PHASE
- [x] intent-capture — Intent Capture — EXECUTE
- [-] requirements-analysis — Requirements Analysis — EXECUTE
- [ ] domain-modeling — Domain Modeling — SKIP
- [?] gated-stage — Gated — EXECUTE
- [R] revising-stage — Revising — EXECUTE
- [S] skipped-stage — Skipped — EXECUTE
not a checkbox line
";

    #[test]
    fn parses_all_six_marker_states_and_ignores_non_matching_lines() {
        let entries = parse_checkboxes(SAMPLE);
        assert_eq!(entries.len(), 6);
        assert_eq!(entries[0].state, CheckboxState::Completed);
        assert_eq!(entries[1].state, CheckboxState::InProgress);
        assert_eq!(entries[2].state, CheckboxState::Pending);
        assert_eq!(entries[3].state, CheckboxState::AwaitingApproval);
        assert_eq!(entries[4].state, CheckboxState::Revising);
        assert_eq!(entries[5].state, CheckboxState::Skipped);
        assert_eq!(entries[0].slug, "intent-capture");
        assert_eq!(entries[2].rest, "Domain Modeling — SKIP");
    }

    #[test]
    fn set_checkbox_edits_only_the_marker_and_preserves_the_rest_verbatim() {
        let updated =
            set_checkbox(SAMPLE, "requirements-analysis", CheckboxState::Completed).unwrap();
        assert!(updated.contains("- [x] requirements-analysis — Requirements Analysis — EXECUTE"));
        // 他の行は不変
        assert!(updated.contains("- [ ] domain-modeling — Domain Modeling — SKIP"));
        assert_eq!(count_completed(&updated), 2);
    }

    #[test]
    fn set_checkbox_refuses_missing_stages() {
        assert_eq!(
            set_checkbox(SAMPLE, "no-such-stage", CheckboxState::Completed),
            Err(CheckboxWriteError::MissingStage("no-such-stage".into()))
        );
    }
}
