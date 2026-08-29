//! `RuleBundleSource` — active-space のルール束 (決定論的 steering) の読取ポート。
//!
//! ルールの**テキスト**は必須 steering であり、パス読みへ降格しない (02 §10)。読み順は
//! memory 層の解決順 (`org → team → project → phase`)。ファイル I/O は実装 (アダプタ層) の
//! 内部詳細で、ポート面には現れない。

use core_command_domain::workflow_definition::PhaseId;

/// ルールファイル 1 つ (パス + 全文)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleFile {
    path: String,
    text: String,
}

impl RuleFile {
    /// パスと全文を束ねる。
    #[must_use]
    pub const fn new(path: String, text: String) -> RuleFile {
        RuleFile { path, text }
    }

    /// ルールファイルのパス (workspace 相対)。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// ルールの全文。
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// 読取の失敗 (blocking — ステージは開始しない)。材料のみ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleBundleReadError {
    /// 必須ルールファイルが読めない (欠落・権限・UTF-8 破損)。
    Unreadable {
        /// 読もうとしたパス。
        path: String,
        /// 失敗の理由 (OS 由来)。
        cause: String,
    },
}

/// active-space のルール束を読む (読取専用)。
pub trait RuleBundleSource {
    /// 実行フェーズに応じたルール束を読み順で返す。空 (ルール未整備) は正常。
    ///
    /// # Errors
    ///
    /// 必須ルールファイルの読取失敗 (`Unreadable`) — 呼出側は run-stage の代わりに
    /// `error` directive を出す (02 §10)。
    fn load(&self, phase: PhaseId) -> Result<Vec<RuleFile>, RuleBundleReadError>;
}
