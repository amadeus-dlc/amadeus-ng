//! 固定本家2.7.1の実装開始判定を、全公開フィールドで比較する。
#![allow(clippy::unwrap_used)]
use core_command_domain::orchestration::*;
use core_infrastructure::canon_json::{JsonValue, ObjectMembers, to_value};
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
            status => {
                assert_eq!(status, "generation");
                PlanGenerationStatus::Generation
            }
        },
    )
    .unwrap();
    PlanReceipts::new([receipt]).unwrap()
}
#[test]
fn source_readiness_observations_match_all_fields() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../../../tests/golden/selfhost-stage1/plan-readiness.json"
    ))
    .unwrap();
    let observations = corpus.get("observations").unwrap().as_array().unwrap();
    assert!(!observations.is_empty());
    let mut ids = std::collections::HashSet::new();
    for case in observations {
        let id = string(case, "id");
        assert!(ids.insert(id));
        let docs = case.get("documents").unwrap();
        let documents = PlanApprovalDocuments::new(
            string(docs, "plan").to_string(),
            string(docs, "instructions").to_string(),
            string(docs, "questions").to_string(),
            string(docs, "questions_file").to_string(),
        );
        let sections = case.get("sections").unwrap();
        let context = case.get("context").unwrap();
        let posture = TestingPosture::resolve(
            &TestingSections::new(
                string(sections, "org").to_string(),
                string(sections, "team").to_string(),
                string(sections, "project").to_string(),
            ),
            &TestingContext::new(
                string(context, "scope").to_string(),
                string(context, "testStrategy").to_string(),
                string(context, "projectType").to_string(),
            ),
        );
        let result = CodeGenerationApproval::evaluate(
            Ok(authority(case.get("authority").unwrap())),
            &PlanTarget::stage_level(),
            &documents,
            posture,
            &receipts(case.get("receipt").unwrap()),
            case.get("current_source").unwrap().as_str(),
        );
        let mut fields = ObjectMembers::new();
        for (key, value) in [
            ("ok", to_value(&result.ok()).unwrap()),
            ("unit", to_value(&result.unit()).unwrap()),
            ("reason", to_value(&result.reason()).unwrap()),
            ("planExists", to_value(&result.plan_exists()).unwrap()),
            (
                "instructionsExist",
                to_value(&result.instructions_exist()).unwrap(),
            ),
            ("approved", to_value(&result.approved()).unwrap()),
            ("contractValid", to_value(&result.contract_valid()).unwrap()),
            (
                "fingerprintValid",
                to_value(&result.fingerprint_valid()).unwrap(),
            ),
            ("receiptValid", to_value(&result.receipt_valid()).unwrap()),
            ("contractHash", to_value(&result.contract_hash()).unwrap()),
            (
                "approvalFingerprint",
                to_value(&result.approval_fingerprint()).unwrap(),
            ),
            (
                "directiveEpoch",
                to_value(&result.directive_epoch()).unwrap(),
            ),
        ] {
            fields.insert(key, value);
        }
        assert_eq!(
            JsonValue::Object(fields),
            to_value(case.get("expected").unwrap()).unwrap(),
            "{id}"
        );
    }
}

#[test]
fn generation_request_checks_current_source_before_publication() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../../../tests/golden/selfhost-stage1/plan-readiness.json"
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
    let (mut runtime, _) = PlanApprovalRuntime::create(at);
    let challenge = PlanChallenge::issue(
        receipt.decision().evidence().clone(),
        receipt.decision().session().clone(),
        ["Approve Plan".to_string(), "Request Changes".to_string()],
        false,
    );
    let issuance = PlanApprovalOperationId::generate();
    runtime
        .issue_challenge(issuance.clone(), challenge.clone(), at)
        .unwrap();
    runtime
        .observe_response(
            PlanApprovalOperationId::generate(),
            &issuance,
            challenge.session(),
            "1",
            at,
        )
        .unwrap();
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
    runtime.record_answer(answer_id.clone(), input, at).unwrap();
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
    runtime
        .complete_answer(
            &answer_id,
            &core_command_domain::workspace::SpaceName::default(),
            &source,
            at,
        )
        .unwrap();
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
    let before = runtime.clone();
    let refused = runtime
        .request_generation(PlanApprovalOperationId::generate(), &approval, None, at)
        .unwrap_err();
    assert_eq!(
        refused.to_string(),
        "workspace source changed after Plan Approval and before generation began"
    );
    assert_eq!(runtime, before);
    let operation = PlanApprovalOperationId::generate();
    runtime
        .request_generation(
            operation.clone(),
            &approval,
            Some(receipt.certified_source()),
            at,
        )
        .unwrap();
    assert_eq!(
        runtime.receipts().get(&receipt.key()).unwrap().status(),
        PlanGenerationStatus::Generation
    );
    assert_eq!(
        runtime.generations().get(&operation).unwrap().state(),
        PlanGenerationState::Pending
    );
    let before_completion = runtime.clone();
    assert!(
        runtime
            .issue_challenge(PlanApprovalOperationId::generate(), challenge.clone(), at)
            .is_err()
    );
    assert_eq!(runtime, before_completion);
    let mut revoked = runtime.clone();
    revoked.certify_generation(&operation, None, at).unwrap();
    assert_eq!(
        revoked.generations().get(&operation).unwrap().state(),
        PlanGenerationState::Revoked
    );
    assert!(revoked.receipts().get(&receipt.key()).is_none());
    runtime
        .certify_generation(&operation, Some(receipt.certified_source()), at)
        .unwrap();
    assert_eq!(
        runtime.generations().get(&operation).unwrap().state(),
        PlanGenerationState::Active
    );
    assert!(
        runtime
            .certify_generation(&operation, Some(receipt.certified_source()), at)
            .is_err()
    );
    let repeated = PlanApprovalOperationId::generate();
    runtime
        .request_generation(repeated.clone(), &approval, None, at)
        .unwrap();
    assert_eq!(
        runtime.generations().get(&repeated).unwrap().state(),
        PlanGenerationState::Active
    );
    assert!(runtime.generations().pending().next().is_none());
}
#[path = "../../../../../tests/support/approval_answer.rs"]
mod approval_answer;
