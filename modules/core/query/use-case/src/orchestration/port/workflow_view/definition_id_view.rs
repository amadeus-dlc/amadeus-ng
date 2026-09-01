//! `DefinitionIdView` — ワークフロー定義の**系譜 ID** (ADR-008)。値は `harness.json` の
//! `name` (出荷ハーネスでは `claude`)。

use super::definition_id_error::DefinitionIdError;

/// このハーネスにインストールされたワークフロー定義の系譜 ID。
///
/// 内容 (ピン更新・プラグイン選択・再コンパイル) が変わっても不変であり、内容の版は
/// [`super::DefinitionRevisionView`] が別に持つ。`Ord` は生文字列の辞書順。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefinitionIdView(String);

impl DefinitionIdView {
    /// 前後の空白を落としてから検証する。
    ///
    /// # Errors
    ///
    /// 空 (空白のみを含む) と、制御文字を含む値を拒否する。
    pub fn parse(s: &str) -> Result<DefinitionIdView, DefinitionIdError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(DefinitionIdError::Empty);
        }
        if let Some(c) = trimmed.chars().find(|c| c.is_control()) {
            return Err(DefinitionIdError::ControlCharacter(c));
        }
        Ok(DefinitionIdView(trimmed.to_string()))
    }

    /// 生の id 文字列 (trim 済み)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_round_trips_the_shipped_harness_name() {
        for raw in ["claude", "kiro", "codex", "opencode", "copilot"] {
            assert_eq!(DefinitionIdView::parse(raw).unwrap().as_str(), raw);
        }
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_before_validation() {
        let id = DefinitionIdView::parse("  claude\n").unwrap();
        assert_eq!(id.as_str(), "claude");
        assert_eq!(id, DefinitionIdView::parse("claude").unwrap());
    }

    #[test]
    fn an_empty_blank_or_control_bearing_name_cannot_be_constructed() {
        assert_eq!(DefinitionIdView::parse(""), Err(DefinitionIdError::Empty));
        assert_eq!(
            DefinitionIdView::parse("   \t\n "),
            Err(DefinitionIdError::Empty)
        );
        assert_eq!(
            DefinitionIdView::parse("cla\u{7}ude"),
            Err(DefinitionIdError::ControlCharacter('\u{7}'))
        );
    }
}
