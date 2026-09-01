//! `ReadOnlyVerb` — 読み取り専用ユーティリティが指す操作の語彙。
//!
//! 変種名は操作の意図から取る (状態報告・使い方・健全性診断・版表示)。CLI 綴りへの写しは
//! [`EngineCommand::cli_spelling`] が 1 点で持つ (逸脱台帳 #1)。
//!
//! [`EngineCommand::cli_spelling`]: super::EngineCommand::cli_spelling

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
