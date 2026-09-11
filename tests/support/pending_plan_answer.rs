//! 受領候補の保存・投影検査で使用する合成履歴。
#![allow(clippy::unwrap_used)]
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{PlanApprovalOperationId, PlanApprovalRuntime};

pub(super) fn pending_answer(
    at: DateTime<Utc>,
) -> (
    PlanApprovalRuntime,
    Vec<core_command_domain::orchestration::PlanApprovalEvent>,
    PlanApprovalOperationId,
) {
    use core_command_domain::orchestration::*;
    use core_command_domain::workspace::SpaceName;
    let (mut runtime, created) = PlanApprovalRuntime::create(at);
    let mut events = vec![created];
    let authority = CodeGenerationAuthority::new(
        &PlanTarget::stage_level(),
        &IntentId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
        format!("sha256:{}", "a".repeat(64)),
        "WORKFLOW_STARTED:2026-09-08T01:00:00Z#1".to_string(),
        "b".repeat(64),
        2,
    )
    .unwrap();
    let evidence = PlanApprovalEvidence::new(
        authority,
        format!("sha256:{}", "c".repeat(64)),
        "questions.md".to_string(),
        "d".repeat(64),
        "e".repeat(64),
    )
    .unwrap();
    let challenge = PlanChallenge::issue(
        evidence.clone(),
        PlanSession::new("session".to_string()).unwrap(),
        ["Approve Plan".to_string(), "Request Changes".to_string()],
        false,
    );
    let occurrence = PlanApprovalOperationId::generate();
    events.push(
        runtime
            .issue_challenge(occurrence.clone(), challenge.clone(), at)
            .unwrap(),
    );
    events.push(
        runtime
            .observe_response(
                PlanApprovalOperationId::generate(),
                &occurrence,
                challenge.session(),
                "1",
                at,
            )
            .unwrap(),
    );
    let id = PlanApprovalOperationId::generate();
    let input = PlanAnswerInput::new(
        PlanApprovalOrigin::new(
            SpaceName::default(),
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0001").unwrap(),
        ),
        "code-generation".to_string(),
        PlanDecisionEvidence::new(evidence, challenge.session().clone()),
        PlanChoice::ApprovePlan,
        Some("b".repeat(64)),
    );
    events.push(runtime.record_answer(id.clone(), input, at).unwrap());
    (runtime, events, id)
}
