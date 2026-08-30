//! `TurnMaterials` — 合成ルートが呼出し前に読み終えた読取素材 (ポートではなく値)。
//!
//! コマンド側の読取は「リポジトリがイベントから集約を作ること」だけであり (オーナー裁定
//! 2026-08-30)、リードモデル (`stage-graph.json` はコンパイルコンテキストのイベント投影) と
//! memory 層ファイルを読めるのは両側を知る**合成ルートだけ**である。そこで定義とルール束は
//! ポートではなく**値**として受ける — [`NextTurnInput`] と同格の入力である (issue #46 —
//! 旧 `WorkflowDefinitionRepository` / `RuleBundleSource` ポートの廃止)。
//!
//! 読取失敗も**値**で運ぶ: 定義は診断済みの逐語文言 (`String` — 組み立ては読んだ側 =
//! 合成ルート + アダプタの文言関数)、ルール束は材料 ([`RuleUnreadable`]) で受け、文言は
//! ユースケースの `wording` が組む (失敗態度の正本は従来どおり各出し手にある)。
//!
//! [`NextTurnInput`]: super::next_turn_input::NextTurnInput

use core_command_domain::orchestration::MemoryRules;
use core_command_domain::workflow_definition::WorkflowDefinition;

/// 必須ルールファイルが読めない (在るのに権限・UTF-8 破損で開けない — blocking)。
///
/// ファイルが**無い**のは正常であり、ここには来ない (loader が黙って読み飛ばす)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleUnreadable {
    path: String,
    cause: String,
}

impl RuleUnreadable {
    /// 読もうとしたパスと OS 由来の理由から組む。
    #[must_use]
    pub const fn new(path: String, cause: String) -> RuleUnreadable {
        RuleUnreadable { path, cause }
    }

    /// 読もうとしたパス。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// 失敗の理由 (OS 由来)。
    #[must_use]
    pub fn cause(&self) -> &str {
        &self.cause
    }
}

/// 1 ターンぶんの読取素材 — ワークフロー定義と memory 層ルール束。
///
/// どちらも「読めた値」か「読取失敗の記述」を運ぶ。失敗でもユースケースは呼ばれる —
/// どの分岐がその素材を**必要とするか**はラダーの判断であり、素材が要らない分岐
/// (読み取り専用ユーティリティなど) は失敗の影響を受けない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnMaterials {
    definition: Result<WorkflowDefinition, String>,
    rules: Result<MemoryRules, RuleUnreadable>,
}

impl TurnMaterials {
    /// 読み終えた素材から組む。
    #[must_use]
    pub const fn new(
        definition: Result<WorkflowDefinition, String>,
        rules: Result<MemoryRules, RuleUnreadable>,
    ) -> TurnMaterials {
        TurnMaterials { definition, rules }
    }

    /// ワークフロー定義。
    ///
    /// # Errors
    ///
    /// 読めなかったときは診断済みの逐語文言 (組み立ては読んだ側 = 合成ルート)。
    pub fn definition(&self) -> Result<&WorkflowDefinition, &str> {
        self.definition.as_ref().map_err(String::as_str)
    }

    /// memory 層のルール束。
    ///
    /// # Errors
    ///
    /// 読めなかったときはその材料 ([`RuleUnreadable`] — 文言はユースケースの `wording`)。
    pub const fn rules(&self) -> Result<&MemoryRules, &RuleUnreadable> {
        self.rules.as_ref()
    }
}
