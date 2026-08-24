//! `CheckboxState` / Stage Progress 行 — 6 状態マーカー＋ em dash 区切り。パース文法は
//! `/^- \[([ xSR?-])\] (\S+)\s*—\s*(.*)$/gm` (upstream `aidlc-lib.ts:6678`, 03 §5.4)。
//! marker writer と suffix writer は同一行の互いに素なフィールドを編集する別 API
//! (recompose と jump が合成できる根拠)。suffix writer の正確な直列化は
//! TODO(golden: stage-0) — 本スライスは marker 側のみ実装する。

/// 6 状態 (01 §3.3 — E1)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckboxState {
    /// `[ ]` = upstream `pending` — 未着手。
    Pending,
    /// `[-]` = upstream `in-progress` — 実行中 (ゲートはまだ開いていない)。
    InProgress,
    /// `[?]` = upstream `awaiting-approval` — 承認ゲート開放済み (`[-]` → `[?]`)。
    AwaitingApproval,
    /// `[R]` = upstream `revising` — 差戻し後の改訂中。ゲートに再入できる唯一の状態。
    Revising,
    /// `[x]` = upstream `completed` — 完了。`Completed` フィールド同期の集計対象。
    Completed,
    /// `[S]` = upstream `skipped` — 経路上の帰結としての読み飛ばし (完了ではない)。
    Skipped,
}

impl CheckboxState {
    /// 行に書かれる 1 文字マーカー。`from_marker` の逆写像であり 6 状態と 1:1
    /// (往復忠実: `from_marker(s.marker()) == Some(s)`)。
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

    /// マーカー 1 文字から状態へ。閉集合 `[ xSR?-]` の外は `None` — 呼出側 (`parse_line`) は
    /// その行を checkbox 行と見なさない。
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

    // ---- 分類述語 (マーカー語彙の分類は本型が所有する — Tell, Don't Ask)。
    //      呼出側で変種集合を再列挙しないこと。 ----

    /// 未終了 (in-flight)。upstream `next` の in-flight 判定 (02 §5.1 手順 10-2) の集合:
    /// `pending / in-progress / awaiting-approval / revising`。
    /// 「checkbox 行の欠落も in-flight」の扱いは行の存否を知る呼出側の責務。
    #[must_use]
    pub const fn is_in_flight(self) -> bool {
        !self.is_finished()
    }

    /// 終了済み (`completed` / `skipped`) — 前進走査 (`nextInScopeStage`) が読み飛ばす集合。
    #[must_use]
    pub const fn is_finished(self) -> bool {
        matches!(self, CheckboxState::Completed | CheckboxState::Skipped)
    }

    /// 着手済み (in-flight のうち `pending` を除く: `in-progress / awaiting-approval /
    /// revising`)。jump forward が skipped 化する「現ステージ」の条件 (09 §3)。
    #[must_use]
    pub const fn is_active(self) -> bool {
        self.is_in_flight() && !matches!(self, CheckboxState::Pending)
    }
}

/// パース済みの Stage Progress 行 — マーカー / stage slug / em dash 以降のテキストの 3 分割。
/// 元の行の空白配置は保持しない (書き戻しは `with_checkbox_marker` が元の行を verbatim に扱う)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckboxEntry {
    state: CheckboxState,
    slug: String,
    /// em dash 以降のテキスト (verbatim 保存 — title や EXECUTE/SKIP サフィックスを含む)。
    rest: String,
}

impl CheckboxEntry {
    /// 3 成分から組む。行文法の検査は行わない (検査済みの値を運ぶ入れ物であり、行の正本は
    /// `parse_checkboxes`)。
    #[must_use]
    pub fn new(
        state: CheckboxState,
        slug: impl Into<String>,
        rest: impl Into<String>,
    ) -> CheckboxEntry {
        CheckboxEntry {
            state,
            slug: slug.into(),
            rest: rest.into(),
        }
    }

    /// マーカーが表す run-state (計画側の EXECUTE/SKIP サフィックスとは別フィールド)。
    #[must_use]
    pub const fn state(&self) -> CheckboxState {
        self.state
    }

    /// stage slug — 行の識別子。空白を含まない 1 トークンで、`with_checkbox_marker` の照合キー。
    #[must_use]
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// em dash 以降のテキスト (verbatim 保存 — title や EXECUTE/SKIP サフィックスを含む)。
    #[must_use]
    pub fn rest(&self) -> &str {
        &self.rest
    }
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
    Some(CheckboxEntry::new(state, slug, tail))
}

/// marker writer (`with_checkbox_marker`) の拒否理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckboxUpdateError {
    /// 対象 slug の行が存在しない。
    MissingStage(String),
}

/// marker のみを書き換える (rest と suffix には触れない — 互いに素なフィールド編集)。
/// # Errors
///
/// 対象 slug の行が存在しなければ `MissingStage`。
pub fn with_checkbox_marker(
    content: &str,
    slug: &str,
    state: CheckboxState,
) -> Result<String, CheckboxUpdateError> {
    let mut found = false;
    let mut out: Vec<String> = Vec::new();
    for line in content.lines() {
        match parse_line(line) {
            Some(entry) if entry.slug() == slug => {
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
        return Err(CheckboxUpdateError::MissingStage(slug.to_string()));
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
        .filter(|e| e.state() == CheckboxState::Completed)
        .count()
}

#[cfg(test)]
mod tests {
    // テストは固定長フィクスチャの添字参照を許容 (clippy.toml に相当設定が無いため file 単位で
    // allow)。
    #![allow(clippy::indexing_slicing)]

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
        assert_eq!(entries[0].state(), CheckboxState::Completed);
        assert_eq!(entries[1].state(), CheckboxState::InProgress);
        assert_eq!(entries[2].state(), CheckboxState::Pending);
        assert_eq!(entries[3].state(), CheckboxState::AwaitingApproval);
        assert_eq!(entries[4].state(), CheckboxState::Revising);
        assert_eq!(entries[5].state(), CheckboxState::Skipped);
        assert_eq!(entries[0].slug(), "intent-capture");
        assert_eq!(entries[2].rest(), "Domain Modeling — SKIP");
    }

    #[test]
    fn with_checkbox_marker_edits_only_the_marker_and_preserves_the_rest_verbatim() {
        let updated =
            with_checkbox_marker(SAMPLE, "requirements-analysis", CheckboxState::Completed)
                .unwrap();
        assert!(updated.contains("- [x] requirements-analysis — Requirements Analysis — EXECUTE"));
        // 他の行は不変
        assert!(updated.contains("- [ ] domain-modeling — Domain Modeling — SKIP"));
        assert_eq!(count_completed(&updated), 2);
    }

    #[test]
    fn with_checkbox_marker_refuses_missing_stages() {
        assert_eq!(
            with_checkbox_marker(SAMPLE, "no-such-stage", CheckboxState::Completed),
            Err(CheckboxUpdateError::MissingStage("no-such-stage".into()))
        );
    }

    #[test]
    fn classification_predicates_partition_the_six_markers() {
        use CheckboxState::{AwaitingApproval, Completed, InProgress, Pending, Revising, Skipped};
        let all = [
            Pending,
            InProgress,
            AwaitingApproval,
            Revising,
            Completed,
            Skipped,
        ];
        for cb in all {
            // in-flight と finished は補集合
            assert_ne!(cb.is_in_flight(), cb.is_finished(), "{cb:?}");
            // active = in-flight ∧ ¬pending
            assert_eq!(cb.is_active(), cb.is_in_flight() && cb != Pending, "{cb:?}");
        }
        assert!(Pending.is_in_flight() && !Pending.is_active());
        assert!(Revising.is_active());
        assert!(Completed.is_finished() && Skipped.is_finished());
    }
}
