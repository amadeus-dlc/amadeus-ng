//! 共有失効の投影/再生検査が使う、元の発行の合成履歴。
#![allow(clippy::unwrap_used)]
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    DirectivePublication, IntentExecution, IntentExecutionEventId, IntentId,
    PlanApprovalOperationId, PlanApprovalRuntime, PublishedDirective, StageDisplay, StageEntries,
    StageEntry, Started,
};
use core_command_domain::workflow_definition::{PhaseId, PlanAction, StageNumber, StageSlug};
use core_command_domain::workspace::SpaceName;
/// テストの準備に記録された対象で、発行済み/未発行の実行履歴を組む。
/// 呼出側から本番の失効コマンドへ真偽値を渡す経路は作らない。
pub(super) fn source_execution(
    runtime: &PlanApprovalRuntime,
    operation: &PlanApprovalOperationId,
    published: bool,
    at: DateTime<Utc>,
) -> (SpaceName, IntentExecution) {
    let pending = runtime
        .invalidations()
        .iter()
        .find(|entry| entry.id() == operation)
        .unwrap();
    let stage = StageSlug::parse("code-generation").unwrap();
    let stages = StageEntries::new(vec![StageEntry::new(
        stage.clone(),
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
    let mut execution = IntentExecution::from((
        Started::new(
            IntentExecutionEventId::generate(),
            pending.execution_id().clone(),
            IntentId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0099").unwrap(),
            stages,
        ),
        at,
    ));
    if published {
        let publication = DirectivePublication::new(
            "a".repeat(64),
            "b".repeat(64),
            PublishedDirective::RunStage { stage, unit: None },
        )
        .with_approval_operation(Some(operation.clone()));
        execution.issue_directive(&publication, at).unwrap();
    }
    (pending.space().clone(), execution)
}
