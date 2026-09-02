//! `ExecutionStageRow` — `read_execution_stage` の 1 行 (実行 × ステージの実行時状態)。

use core_command_domain::orchestration::{Intent, IntentExecution, StageIndex, StageKey};
use core_command_domain::workflow_definition::PlanAction;

use super::spelling;

/// `read_execution_stage` の 1 行。主キーは (`execution_id`, `stage_index`)。
///
/// 値はすべて集約のステージ単位クエリの答えである — `checkbox` / `effective_plan` /
/// `approved` / `revision_count` / `gated`。**実効プランは静的グリッドではない**
/// (recompose のオーバレイが勝つ) ので、`read_intent_stage.plan_action` とは別の列である。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionStageRow {
    execution_id: String,
    stage_index: usize,
    slug: String,
    phase: String,
    checkbox: Option<String>,
    effective_plan: Option<String>,
    approved: Option<bool>,
    revision_count: Option<u32>,
    gated: Option<bool>,
}

impl ExecutionStageRow {
    /// 実行 × ステージの 1 セルを 1 行へ写す (**この型の唯一の構築経路**)。
    ///
    /// `key` は集約の添字帳から引いたそのステージの鍵である (位置と鍵の対応を行の側で
    /// 組み直さない)。
    #[must_use]
    pub fn of(
        execution: &IntentExecution,
        intent: &Intent,
        stage: StageIndex,
        key: &StageKey,
    ) -> ExecutionStageRow {
        ExecutionStageRow {
            execution_id: execution.id().as_str().to_string(),
            stage_index: stage.to_usize(),
            slug: key.slug().as_str().to_string(),
            phase: key.phase().as_str().to_string(),
            checkbox: execution
                .checkbox(stage)
                .map(spelling::checkbox)
                .map(str::to_string),
            effective_plan: execution
                .effective_plan(stage)
                .map(PlanAction::as_str)
                .map(str::to_string),
            approved: execution.approved(stage),
            revision_count: execution.revision_count(stage),
            gated: execution.gated(intent, stage),
        }
    }

    /// 実行の識別子。
    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// 文書順の位置 (0 始まり)。
    #[must_use]
    pub const fn stage_index(&self) -> usize {
        self.stage_index
    }

    /// ステージの slug。
    #[must_use]
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// フェーズの綴り。
    #[must_use]
    pub fn phase(&self) -> &str {
        &self.phase
    }

    /// 観測 checkbox の綴り (`pending` … `skipped`)。
    #[must_use]
    pub fn checkbox(&self) -> Option<&str> {
        self.checkbox.as_deref()
    }

    /// 実効プラン (recompose のオーバレイ反映後)。
    #[must_use]
    pub fn effective_plan(&self) -> Option<&str> {
        self.effective_plan.as_deref()
    }

    /// ゲートが承認済みか。
    #[must_use]
    pub const fn approved(&self) -> Option<bool> {
        self.approved
    }

    /// 差し戻し回数。
    #[must_use]
    pub const fn revision_count(&self) -> Option<u32> {
        self.revision_count
    }

    /// このステージが承認ゲートを要するか。
    #[must_use]
    pub const fn gated(&self) -> Option<bool> {
        self.gated
    }
}
