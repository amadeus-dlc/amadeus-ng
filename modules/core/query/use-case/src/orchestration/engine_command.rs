//! `EngineCommand` — `next` が人間・conductor へ名指しするエンジンコマンドの概念と綴り。
//!
//! 概念 (どの操作を指しているか) と綴り (upstream 3 形のうち self-host 正準の
//! **素のマルチコール形** — 例 `aidlc-utility status`、`07-hooks.md:260` に実在し ADR 0002
//! 決定 3) はどちらも読み手の閉じた出力語彙である。綴りの導出は CPU とメモリだけの純計算
//! なのでポートにしない。ディスパッチャ語彙の完全 ROUTES 写し (30 経路 +
//! SLASH_FLAG_ALIASES) は U7 / A1 で表として実体化し、差し替えは
//! [`EngineCommand::cli_spelling`] 1 点で行う (逸脱台帳 #1)。

use crate::workflow_view::{ScopeSlugView, StageSlugView};

/// 読み取り専用ユーティリティの語彙 (分岐 1 — `--status` などのフラグが指す操作)。
///
/// 変種名は操作の意図から取る (状態報告・使い方・健全性診断・版表示)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadOnlyVerb {
    /// ワークフロー状態の報告。
    Status,
    /// 使い方の表示。
    Help,
    /// セットアップ健全性の診断。
    Doctor,
    /// フレームワーク版の表示。
    Version,
}

/// 設定変更の対象フィールド (分岐 5 の閉集合)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigField {
    /// 深さ (`depth`)。
    Depth,
    /// テスト戦略 (`test-strategy`)。
    TestStrategy,
    /// レビュー上限 (`review`)。
    Review,
}

/// `next` が名指しするエンジンコマンドの閉じた語彙。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineCommand {
    /// 読み取り専用ユーティリティ (分岐 1)。
    ReadOnlyUtility(ReadOnlyVerb),
    /// 名詞トークン列の逐語通し (分岐 1b/1c/1d — 人間の語をそのまま運ぶ)。
    NounTokens(Vec<String>),
    /// park 解除 (分岐 2.6)。
    Unpark,
    /// jump の純読み取り解決 (分岐 7)。
    ResolveJump {
        /// ジャンプ先ステージ。
        stage: StageSlugView,
    },
    /// intent の鋳造 (birth — `next` は自身で実行しない)。
    MintIntent {
        /// 鋳造する intent の scope。
        scope: ScopeSlugView,
    },
    /// scope 変更の名指し (分岐 5)。
    ChangeScope {
        /// 変更先 scope。
        scope: ScopeSlugView,
    },
    /// depth / test-strategy / review の設定変更の名指し (分岐 5)。
    ChangeConfig {
        /// 対象フィールド。
        field: ConfigField,
        /// 人間が与えた値 (逐語)。
        value: String,
    },
    /// composer ディスパッチの名指し (分岐 4c)。
    DispatchComposer,
}

impl EngineCommand {
    /// コマンド概念を CLI 綴り (マルチコール正準形) に写す。
    #[must_use]
    pub fn cli_spelling(&self) -> String {
        match self {
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

    #[test]
    fn commands_compare_by_value() {
        assert_eq!(
            EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Status),
            EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Status)
        );
        assert_ne!(EngineCommand::Unpark, EngineCommand::DispatchComposer);
        assert_eq!(
            EngineCommand::ChangeConfig {
                field: ConfigField::Depth,
                value: "standard".to_string(),
            },
            EngineCommand::ChangeConfig {
                field: ConfigField::Depth,
                value: "standard".to_string(),
            }
        );
    }

    #[test]
    fn every_command_concept_spells_in_multicall_form() {
        assert_eq!(
            EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Status).cli_spelling(),
            "aidlc-utility status"
        );
        assert_eq!(
            EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Help).cli_spelling(),
            "aidlc-utility help"
        );
        assert_eq!(
            EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Doctor).cli_spelling(),
            "aidlc-utility doctor"
        );
        assert_eq!(
            EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Version).cli_spelling(),
            "aidlc-utility version"
        );
        assert_eq!(
            EngineCommand::NounTokens(vec!["intent".to_string(), "list".to_string()])
                .cli_spelling(),
            "aidlc-utility intent list"
        );
        assert_eq!(EngineCommand::Unpark.cli_spelling(), "aidlc-state unpark");
        assert_eq!(
            EngineCommand::ResolveJump {
                stage: StageSlugView::parse("domain-design").unwrap(),
            }
            .cli_spelling(),
            "aidlc-jump resolve --stage domain-design"
        );
        assert_eq!(
            EngineCommand::MintIntent {
                scope: ScopeSlugView::parse("bugfix").unwrap(),
            }
            .cli_spelling(),
            "aidlc-utility intent-create --scope bugfix --label \"<2-3 word kebab essence>\""
        );
        assert_eq!(
            EngineCommand::ChangeScope {
                scope: ScopeSlugView::parse("mvp").unwrap(),
            }
            .cli_spelling(),
            "aidlc-utility scope-change --scope mvp"
        );
        assert_eq!(
            EngineCommand::ChangeConfig {
                field: ConfigField::Depth,
                value: "standard".to_string(),
            }
            .cli_spelling(),
            "aidlc-utility config-change --depth standard"
        );
        assert_eq!(
            EngineCommand::ChangeConfig {
                field: ConfigField::TestStrategy,
                value: "minimal".to_string(),
            }
            .cli_spelling(),
            "aidlc-utility config-change --test-strategy minimal"
        );
        assert_eq!(
            EngineCommand::ChangeConfig {
                field: ConfigField::Review,
                value: "advisory".to_string(),
            }
            .cli_spelling(),
            "aidlc-utility config-change --review advisory"
        );
        assert_eq!(
            EngineCommand::DispatchComposer.cli_spelling(),
            "aidlc-composer detect"
        );
    }
}
