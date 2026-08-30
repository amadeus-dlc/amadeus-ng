//! `RuleBundleSource` — active-space のルール束 (決定論的 steering) の読取ポート。
//!
//! ルールの**テキスト**は必須 steering であり、パス読みへ降格しない (02 §10)。読み順は
//! memory 層の解決順 (`org → team → project → phase`)。ファイル I/O と**分割・パック**
//! (Markdown 見出し境界・輸送上限 — 形式と輸送の知識) は実装 (アダプタ層) の内部詳細で、
//! ポート面には現れない — ポートは分割済みの配信計画 [`SteeringPlan`] を返す。

use core_command_domain::orchestration::SteeringPlan;
use core_command_domain::workflow_definition::PhaseId;

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
    /// セクションを輸送上限未満へ分割できない。
    Unsplittable {
        /// 該当セクションを含むルールファイルのパス。
        path: String,
    },
}

/// active-space のルール束を読み、配信計画に組む (読取専用)。
pub trait RuleBundleSource {
    /// 実行フェーズに応じたルール束を読み順で配信計画に組んで返す。空 (ルール未整備) は
    /// 空計画で正常。
    ///
    /// # Errors
    ///
    /// 必須ルールファイルの読取失敗 (`Unreadable`)・分割不能セクション (`Unsplittable`) —
    /// 呼出側は run-stage の代わりに `error` directive を出す (02 §10)。
    fn load(&self, phase: PhaseId) -> Result<SteeringPlan, RuleBundleReadError>;
}
