//! `CommandSpelling` の実装 — コマンド概念をマルチコール綴りに写すカタログ。
//!
//! upstream 3 形 (bun 直接形 / 素のマルチコール形 / ディスパッチャ形) のうち、self-host の
//! 正準として**素のマルチコール形** (例 `aidlc-utility status` — `07-hooks.md:260` に実在し、
//! バイナリが busybox 式マルチコールで受ける — ADR 0002 決定 3) を使う。ディスパッチャ語彙の
//! 完全 ROUTES 写し (30 経路 + SLASH_FLAG_ALIASES) は U7 / A1 で表として実体化し、差し替えは
//! 本実装 1 点で行う (逸脱台帳 #1)。

use core_command_domain::orchestration::{ConfigField, EngineCommand, ReadOnlyVerb};
use core_command_use_case::orchestration::CommandSpelling;

/// マルチコール形の綴りカタログ。
#[derive(Debug, Clone, Copy, Default)]
pub struct MulticallCommandSpelling;

impl CommandSpelling for MulticallCommandSpelling {
    fn spell(&self, command: &EngineCommand) -> String {
        match command {
            EngineCommand::ReadOnlyUtility(verb) => {
                format!("aidlc-utility {}", read_only_subcommand(*verb))
            }
            EngineCommand::NounTokens(tokens) => {
                format!("aidlc-utility {}", tokens.join(" "))
            }
            EngineCommand::Unpark => "aidlc-state unpark".to_string(),
            EngineCommand::ResolveJump { stage } => {
                format!("aidlc-jump resolve --stage {}", stage.as_str())
            }
            // ラベルは conductor が置換するプレースホルダ付き。
            EngineCommand::MintIntent { scope } => format!(
                "aidlc-utility intent-create --scope {} --label \"<2-3 word kebab essence>\"",
                scope.as_str()
            ),
            EngineCommand::ChangeScope { scope } => {
                format!("aidlc-utility scope-change --scope {}", scope.as_str())
            }
            EngineCommand::ChangeConfig { field, value } => {
                format!(
                    "aidlc-utility config-change --{} {value}",
                    config_flag(*field)
                )
            }
            EngineCommand::DispatchComposer => "aidlc-composer detect".to_string(),
        }
    }
}

/// 読み取り専用ユーティリティのサブコマンド綴り。
const fn read_only_subcommand(verb: ReadOnlyVerb) -> &'static str {
    match verb {
        ReadOnlyVerb::Status => "status",
        ReadOnlyVerb::Help => "help",
        ReadOnlyVerb::Doctor => "doctor",
        ReadOnlyVerb::Version => "version",
    }
}

/// 設定変更フラグの綴り。
const fn config_flag(field: ConfigField) -> &'static str {
    match field {
        ConfigField::Depth => "depth",
        ConfigField::TestStrategy => "test-strategy",
        ConfigField::Review => "review",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_command_domain::workflow_definition::{ScopeSlug, StageSlug};

    #[test]
    fn every_command_concept_spells_in_multicall_form() {
        let spelling = MulticallCommandSpelling;
        assert_eq!(
            spelling.spell(&EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Status)),
            "aidlc-utility status"
        );
        assert_eq!(
            spelling.spell(&EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Help)),
            "aidlc-utility help"
        );
        assert_eq!(
            spelling.spell(&EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Doctor)),
            "aidlc-utility doctor"
        );
        assert_eq!(
            spelling.spell(&EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Version)),
            "aidlc-utility version"
        );
        assert_eq!(
            spelling.spell(&EngineCommand::NounTokens(vec![
                "intent".to_string(),
                "list".to_string(),
            ])),
            "aidlc-utility intent list"
        );
        assert_eq!(spelling.spell(&EngineCommand::Unpark), "aidlc-state unpark");
        assert_eq!(
            spelling.spell(&EngineCommand::ResolveJump {
                stage: StageSlug::parse("domain-design").unwrap(),
            }),
            "aidlc-jump resolve --stage domain-design"
        );
        assert_eq!(
            spelling.spell(&EngineCommand::MintIntent {
                scope: ScopeSlug::parse("bugfix").unwrap(),
            }),
            "aidlc-utility intent-create --scope bugfix --label \"<2-3 word kebab essence>\""
        );
        assert_eq!(
            spelling.spell(&EngineCommand::ChangeScope {
                scope: ScopeSlug::parse("mvp").unwrap(),
            }),
            "aidlc-utility scope-change --scope mvp"
        );
        assert_eq!(
            spelling.spell(&EngineCommand::ChangeConfig {
                field: ConfigField::Depth,
                value: "standard".to_string(),
            }),
            "aidlc-utility config-change --depth standard"
        );
        assert_eq!(
            spelling.spell(&EngineCommand::ChangeConfig {
                field: ConfigField::TestStrategy,
                value: "minimal".to_string(),
            }),
            "aidlc-utility config-change --test-strategy minimal"
        );
        assert_eq!(
            spelling.spell(&EngineCommand::ChangeConfig {
                field: ConfigField::Review,
                value: "advisory".to_string(),
            }),
            "aidlc-utility config-change --review advisory"
        );
        assert_eq!(
            spelling.spell(&EngineCommand::DispatchComposer),
            "aidlc-composer detect"
        );
    }
}
