//! `EngineCommand` — `next` が人間・conductor へ名指しするエンジンコマンドの概念。
//!
//! **概念** (どの操作を指しているか) はドメインの閉じた語彙で、**綴り** (upstream 3 形の
//! どれで書くか — マルチコール形など) はアダプタ層の `CommandSpelling` 実装が持つ
//! (逸脱台帳 #1: 差し替えはアダプタ 1 点)。

use crate::workflow_definition::{ScopeSlug, StageSlug};

/// 読み取り専用ユーティリティの語彙 (分岐 1 — `--status` などのフラグが指す操作)。
///
/// 変種名は操作の意図から取る (状態報告・使い方・健全性診断・版表示)。CLI 綴りへの写像は
/// アダプタ層が持つ。
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
        stage: StageSlug,
    },
    /// intent の鋳造 (birth — `next` は自身で実行しない)。
    MintIntent {
        /// 鋳造する intent の scope。
        scope: ScopeSlug,
    },
    /// scope 変更の名指し (分岐 5)。
    ChangeScope {
        /// 変更先 scope。
        scope: ScopeSlug,
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
}
