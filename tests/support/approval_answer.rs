//! 計画回答の監査配送を検査する合成実行。
#![allow(clippy::unwrap_used)]
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    IntentExecution, IntentExecutionEventId, IntentId, PlanApprovalOperationId,
    PlanApprovalRuntime, StageDisplay, StageEntries, StageEntry, Started,
};
use core_command_domain::workflow_definition::{PhaseId, PlanAction, StageNumber, StageSlug};
pub(super) fn source_execution(
    runtime: &PlanApprovalRuntime,
    id: &PlanApprovalOperationId,
    at: DateTime<Utc>,
) -> IntentExecution {
    let answer = runtime.answers().get(id).unwrap();
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
            answer.input().origin().execution_id().clone(),
            IntentId::parse(answer.input().decision().evidence().authority().intent_id()).unwrap(),
            stages,
        ),
        at,
    ))
}
