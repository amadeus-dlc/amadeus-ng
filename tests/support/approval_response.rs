//! 応答配送を検査する合成実行の履歴。
#![allow(clippy::unwrap_used)]
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    IntentExecution, IntentExecutionEventId, IntentId, PlanApprovalOperationId,
    PlanApprovalRuntime, StageDisplay, StageEntries, StageEntry, Started,
};
use core_command_domain::workflow_definition::{PhaseId, PlanAction, StageNumber, StageSlug};
/// 応答準備の観測先を持つ、合成された実行履歴。
pub(super) fn response_execution(
    runtime: &PlanApprovalRuntime,
    operation: &PlanApprovalOperationId,
    at: DateTime<Utc>,
) -> IntentExecution {
    let pending = runtime.pending_responses().get(operation).unwrap();
    let stage = StageSlug::parse("code-generation").unwrap();
    let stages = StageEntries::new(vec![StageEntry::new(
        stage,
        PhaseId::Construction,
        PlanAction::Execute,
        false,
        StageDisplay::new(
            StageNumber::parse("3.5").unwrap(),
            "Code Generation",
            "developer",
        )
        .unwrap(),
    )])
    .unwrap();
    IntentExecution::from((
        Started::new(
            IntentExecutionEventId::generate(),
            pending.origin().execution_id().clone(),
            IntentId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0099").unwrap(),
            stages,
        ),
        at,
    ))
}
