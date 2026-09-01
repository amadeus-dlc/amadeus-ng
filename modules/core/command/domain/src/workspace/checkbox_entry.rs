//! `CheckboxEntry` — パース済みの Stage Progress 行 (マーカー / stage slug / em dash 以降)。

use super::checkbox_state::CheckboxState;

/// パース済みの Stage Progress 行 — マーカー / stage slug / em dash 以降のテキストの 3 分割。
/// 元の行の空白配置は保持しない (書き戻しは `Checkboxes::with_marker` が元の行を verbatim に扱う)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckboxEntry {
    state: CheckboxState,
    slug: String,
    /// em dash 以降のテキスト (verbatim 保存 — title や EXECUTE/SKIP サフィックスを含む)。
    rest: String,
}

impl CheckboxEntry {
    /// 3 成分から組む。行文法の検査は行わない (検査済みの値を運ぶ入れ物であり、行の正本は
    /// `Checkboxes::parse`)。
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

    /// stage slug — 行の識別子。空白を含まない 1 トークンで、`Checkboxes::with_marker` の照合キー。
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

impl CheckboxEntry {
    /// 1 行のパース (行文法の正本)。文法に一致しない行は `None`。
    pub(super) fn parse_line(line: &str) -> Option<CheckboxEntry> {
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
}
