//! 開始候補の保存検査で使用する、固定観測からの合成履歴。
#![allow(clippy::unwrap_used)]
use core_command_domain::orchestration::*;
use serde_json::Value;
fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).unwrap().as_str().unwrap()
}
fn authority(value: &Value) -> CodeGenerationAuthority {
    CodeGenerationAuthority::new(
        &PlanTarget::stage_level(),
        &IntentId::parse(string(value, "intentId")).unwrap(),
        string(value, "directiveEpoch").to_string(),
        string(value, "runFloor").to_string(),
        string(value, "sourceFloor").to_string(),
        value.get("markerRevision").unwrap().as_u64().unwrap(),
    )
    .unwrap()
}
fn receipts(value: &Value) -> PlanReceipts {
    if value.is_null() {
        return PlanReceipts::default();
    }
    let evidence = PlanApprovalEvidence::new(
        authority(value),
        string(value, "fingerprint").to_string(),
        string(value, "questionsFile").to_string(),
        string(value, "questionsSha256").to_string(),
        string(value, "promptSha256").to_string(),
    )
    .unwrap();
    let receipt = PlanApprovalReceipt::new(
        PlanDecisionEvidence::new(
            evidence,
            PlanSession::new(string(value, "session").to_string()).unwrap(),
        ),
        string(value, "challengeId").to_string(),
        string(value, "certifiedSourceSha256").to_string(),
        match string(value, "status") {
            "approved" => PlanGenerationStatus::Approved,
            "generation" => PlanGenerationStatus::Generation,
            value => {
                assert_eq!(value, "approved");
                PlanGenerationStatus::Approved
            }
        },
    )
    .unwrap();
    PlanReceipts::new([receipt]).unwrap()
}

pub(super) fn pending() -> (
    PlanApprovalRuntime,
    Vec<PlanApprovalEvent>,
    PlanApprovalOperationId,
    chrono::DateTime<chrono::Utc>,
) {
    let corpus: Value = serde_json::from_str(include_str!(
        "../golden/selfhost-stage1/plan-readiness.json"
    ))
    .unwrap();
    let case = corpus
        .get("observations")
        .unwrap()
        .as_array()
        .unwrap()
        .first()
        .unwrap();
    let receipt_set = receipts(case.get("receipt").unwrap());
    let receipt = receipt_set.iter().next().unwrap();
    let at = chrono::DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let (mut runtime, created) = PlanApprovalRuntime::create(at);
    let mut events = vec![created];
    let challenge = PlanChallenge::issue(
        receipt.decision().evidence().clone(),
        receipt.decision().session().clone(),
        ["Approve Plan".to_string(), "Request Changes".to_string()],
        false,
    );
    let issuance = PlanApprovalOperationId::generate();
    let event = runtime
        .issue_challenge(issuance.clone(), challenge.clone(), at)
        .unwrap();
    events.push(event);
    let event = runtime
        .observe_response(
            PlanApprovalOperationId::generate(),
            &issuance,
            challenge.session(),
            "1",
            at,
        )
        .unwrap();
    events.push(event);
    let answer_id = PlanApprovalOperationId::generate();
    let input = PlanAnswerInput::new(
        PlanApprovalOrigin::new(
            core_command_domain::workspace::SpaceName::default(),
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0001").unwrap(),
        ),
        "code-generation".to_string(),
        receipt.decision().clone(),
        PlanChoice::ApprovePlan,
        Some(receipt.certified_source().to_string()),
    );
    let event = runtime.record_answer(answer_id.clone(), input, at).unwrap();
    events.push(event);
    let mut source = approval_answer::source_execution(&runtime, &answer_id, at);
    source
        .record_plan_answer(
            &runtime,
            &answer_id,
            &core_command_domain::workspace::SpaceName::default(),
            Some(receipt.certified_source()),
            at,
        )
        .unwrap();
    let event = runtime
        .complete_answer(
            &answer_id,
            &core_command_domain::workspace::SpaceName::default(),
            &source,
            at,
        )
        .unwrap();
    events.push(event);
    let docs = case.get("documents").unwrap();
    let documents = PlanApprovalDocuments::new(
        string(docs, "plan").to_string(),
        string(docs, "instructions").to_string(),
        string(docs, "questions").to_string(),
        string(docs, "questions_file").to_string(),
    );
    let current = TestingPosture::resolve(
        &TestingSections::new(String::new(), String::new(), String::new()),
        &TestingContext::new(
            "bugfix".to_string(),
            "minimal".to_string(),
            "greenfield".to_string(),
        ),
    );
    let approval = CodeGenerationApproval::evaluate(
        Ok(authority(case.get("authority").unwrap())),
        &PlanTarget::stage_level(),
        &documents,
        current,
        runtime.receipts(),
        Some(receipt.certified_source()),
    );
    assert!(approval.ok());
    let id = PlanApprovalOperationId::generate();
    let event = runtime
        .request_generation(id.clone(), &approval, Some(receipt.certified_source()), at)
        .unwrap();
    events.push(event);
    (runtime, events, id, at)
}
use super::approval_answer;
