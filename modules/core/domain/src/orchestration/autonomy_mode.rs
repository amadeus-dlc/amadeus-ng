//! `AutonomyMode` — 2 境界型 (10 §2.2、PR #2 レビューで確定)。
//! 状態読取側: 正確な `"autonomous"` のみ autonomous、それ以外 (未設定・空・未知値) は
//! すべて gated の fail-closed **リーダ** (初期化は失敗しない)。
//! CLI 引数側: `AutonomyModeArg` の 2 値厳密パースで、不正値は upstream 逐語で拒否
//! (1 型に畳むと本家の拒否文言が発生不能になる)。

use message_catalog::bolt as msg;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AutonomyMode {
    Autonomous,
    Gated,
}

impl AutonomyMode {
    /// 状態ファイル読取の fail-closed リーダ — 「Autonomy is NEVER inferred」の読み側。
    #[must_use]
    pub fn read_state(field_value: Option<&str>) -> AutonomyMode {
        match field_value {
            Some("autonomous") => AutonomyMode::Autonomous,
            _ => AutonomyMode::Gated,
        }
    }

    #[must_use]
    pub fn is_autonomous(self) -> bool {
        self == AutonomyMode::Autonomous
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidModeArg {
    /// upstream 逐語の拒否文言 (文言カタログ経由)。
    message: String,
}

impl InvalidModeArg {
    #[must_use]
    pub fn new(message: impl Into<String>) -> InvalidModeArg {
        InvalidModeArg {
            message: message.into(),
        }
    }

    /// upstream 逐語の拒否文言。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// CLI `--mode` 引数の厳密パース (Controller 規約: 値オブジェクト初期化がバリデーション)。
/// # Errors
///
/// 2 値 (`autonomous` / `gated`) 以外は upstream 逐語 (`Invalid --mode: …`) で拒否する。
pub fn parse_mode_arg(s: &str) -> Result<AutonomyMode, InvalidModeArg> {
    match s {
        "autonomous" => Ok(AutonomyMode::Autonomous),
        "gated" => Ok(AutonomyMode::Gated),
        other => Err(InvalidModeArg::new(msg::invalid_mode(other))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_reader_is_fail_closed_and_never_fails() {
        assert_eq!(
            AutonomyMode::read_state(Some("autonomous")),
            AutonomyMode::Autonomous
        );
        assert_eq!(
            AutonomyMode::read_state(Some("Autonomous")),
            AutonomyMode::Gated
        );
        assert_eq!(AutonomyMode::read_state(Some("")), AutonomyMode::Gated);
        assert_eq!(AutonomyMode::read_state(Some("turbo")), AutonomyMode::Gated);
        assert_eq!(AutonomyMode::read_state(None), AutonomyMode::Gated);
    }

    #[test]
    fn cli_arg_parse_is_strict_with_the_verbatim_rejection() {
        assert_eq!(parse_mode_arg("autonomous"), Ok(AutonomyMode::Autonomous));
        assert_eq!(parse_mode_arg("gated"), Ok(AutonomyMode::Gated));
        let err = parse_mode_arg("turbo").unwrap_err();
        assert_eq!(
            err.message(),
            "Invalid --mode: turbo. Must be 'autonomous' or 'gated'."
        );
    }
}
