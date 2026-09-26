//! `read_execution_stage` の行を組む投影 — 実行 × ステージの 1 セルを [`ExecutionStageRow`] へ
//! 写す。

use core_command_domain::orchestration::{Intent, IntentExecution, StageIndex, StageKey};
use core_command_domain::workflow_definition::PlanAction;

use super::row_id;
use super::spelling;
use crate::orchestration::ExecutionStageRow;

/// 実行 × ステージの 1 セルを 1 行へ写す。
///
/// `key` は集約の添字帳から引いたそのステージの鍵である (位置と鍵の対応を行の側で
/// 組み直さない)。
pub(super) fn row(
    execution: &IntentExecution,
    intent: &Intent,
    stage: StageIndex,
    key: &StageKey,
) -> ExecutionStageRow {
    ExecutionStageRow::new(
        row_id::execution_stage(execution.id().as_str(), stage.to_usize()),
        execution.id().as_str().to_string(),
        stage.to_usize(),
        key.slug().as_str().to_string(),
        key.phase().as_str().to_string(),
        execution
            .checkbox(stage)
            .map(spelling::checkbox)
            .map(str::to_string),
        execution
            .effective_plan(stage)
            .map(PlanAction::as_str)
            .map(str::to_string),
        execution.approved(stage),
        execution.revision_count(stage),
        execution.gated(intent, stage),
    )
}
