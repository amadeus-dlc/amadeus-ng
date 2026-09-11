//! 共有受領のイベントから公開ファイルと操作行を再構成する契約。
#![allow(clippy::unwrap_used)]
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::*;
use core_command_domain::workspace::SpaceName;
use core_read_model_updater::orchestration::PlanApprovalJournalEntry;
use core_read_model_updater::read_tables::PlanApprovalTables;
#[test]
fn pending_answer_projects_receipt_bytes_and_recovery_origin() {
    let at = DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let (mut runtime, created) = PlanApprovalRuntime::create(at);
    let mut entries = vec![PlanApprovalJournalEntry::new(1, at, created)];
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
    let event = runtime
        .issue_challenge(occurrence.clone(), challenge.clone(), at)
        .unwrap();
    entries.push(PlanApprovalJournalEntry::new(2, at, event));
    let event = runtime
        .observe_response(
            PlanApprovalOperationId::generate(),
            &occurrence,
            challenge.session(),
            "1",
            at,
        )
        .unwrap();
    entries.push(PlanApprovalJournalEntry::new(3, at, event));
    let id = PlanApprovalOperationId::generate();
    let execution = IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0001").unwrap();
    let input = PlanAnswerInput::new(
        PlanApprovalOrigin::new(SpaceName::default(), execution.clone()),
        "code-generation".to_string(),
        PlanDecisionEvidence::new(evidence, challenge.session().clone()),
        PlanChoice::ApprovePlan,
        Some("b".repeat(64)),
    );
    let event = runtime.record_answer(id.clone(), input, at).unwrap();
    entries.push(PlanApprovalJournalEntry::new(4, at, event));
    let tables = PlanApprovalTables::project(&entries).unwrap();
    let row = tables
        .rows()
        .iter()
        .find(|row| row.id() == id.as_str())
        .expect("監査配送待ちの操作行");
    assert_eq!(row.status(), "prepared");
    assert_eq!(row.kind(), "answer");
    assert_eq!(row.space(), Some("default"));
    assert_eq!(row.execution_id(), Some(execution.as_str()));
    let receipt = runtime.answers().get(&id).unwrap().receipt().unwrap();
    let filename = format!("receipt-{}.json", receipt.key());
    let file = tables
        .files()
        .iter()
        .find(|file| file.name() == filename)
        .expect("受領ファイル");
    let expected = format!(
        "{{\n  \"version\": 1,\n  \"targetId\": \"stage:code-generation\",\n  \"intentId\": \"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000\",\n  \"directiveEpoch\": \"sha256:{}\",\n  \"runFloor\": \"WORKFLOW_STARTED:2026-09-08T01:00:00Z#1\",\n  \"fingerprint\": \"sha256:{}\",\n  \"questionsFile\": \"questions.md\",\n  \"promptSha256\": \"{}\",\n  \"sourceFloor\": \"{}\",\n  \"markerRevision\": 2,\n  \"session\": \"session\",\n  \"challengeId\": \"{}\",\n  \"choice\": \"Approve Plan\",\n  \"questionsSha256\": \"{}\",\n  \"certifiedSourceSha256\": \"{}\",\n  \"status\": \"approved\"\n}}\n",
        "a".repeat(64),
        "c".repeat(64),
        "e".repeat(64),
        "b".repeat(64),
        challenge.id(),
        "d".repeat(64),
        "b".repeat(64),
    );
    assert_eq!(file.content(), expected);
}
