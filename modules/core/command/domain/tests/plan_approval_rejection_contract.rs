//! 計画承認の値オブジェクトと権限解決の拒否経路 — 逐語文言を契約として固定する。
//!
//! 拒否文言は本家 (ピン `a277af21`) の逐語であり、出す側がそのまま利用者へ渡す。
//! ここでは不正入力が**受理されない**ことと、その材料の綴りを検証する。
#![allow(clippy::unwrap_used)]
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    ActiveDirective, CodeGenerationAuthority, CodeGenerationRunFloor, DirectivePublication,
    IntentId, PlanApprovalDocuments, PlanApprovalEventId, PlanApprovalEvidence,
    PlanApprovalOperationId, PlanOfferedOptions, PlanQuestions, PlanSession, PlanTarget,
    PublishedDirective, RunBoundaryKind, TestingContext, TestingPosture, TestingSections,
};
use core_command_domain::workflow_definition::StageSlug;
use core_command_domain::workspace::{HookHealthError, HookHealthTarget};
use serde_json::Value;

// --- PlanTarget ---------------------------------------------------------------

#[test]
fn a_stage_level_target_has_no_unit_and_the_public_stage_id() {
    let target = PlanTarget::stage_level();
    assert_eq!(target.unit(), None);
    assert_eq!(target.id(), "stage:code-generation");
}

#[test]
fn a_unit_target_is_trimmed_and_named_by_its_unit() {
    let target = PlanTarget::for_unit("  u2-workflow-authority ").unwrap();
    assert_eq!(target.unit(), Some("u2-workflow-authority"));
    assert_eq!(target.id(), "unit:u2-workflow-authority");
}

#[test]
fn an_empty_unit_name_is_rejected() {
    assert_eq!(
        PlanTarget::for_unit("   ").unwrap_err().to_string(),
        "Unit name is empty"
    );
}

#[test]
fn a_unit_name_over_64_chars_is_rejected_with_a_32_char_prefix() {
    let long = "a".repeat(65);
    assert_eq!(
        PlanTarget::for_unit(&long).unwrap_err().to_string(),
        format!("Unit name \"{}...\" is 65 chars; max is 64", "a".repeat(32))
    );
}

#[test]
fn a_unit_name_outside_the_safe_ascii_component_is_rejected() {
    for raw in ["-leading", "with/slash", "日本語"] {
        assert_eq!(
            PlanTarget::for_unit(raw).unwrap_err().to_string(),
            format!(
                "Invalid Unit name \"{raw}\" - must match /^[A-Za-z0-9][A-Za-z0-9._-]*$/ (ASCII letter/digit, then ASCII letters/digits/dot/underscore/hyphen)"
            )
        );
    }
}

// --- PlanSession ---------------------------------------------------------------

#[test]
fn a_session_keeps_its_raw_name_and_folds_unsafe_runs_into_one_hyphen() {
    let session = PlanSession::new("stage1 session/#1".to_string()).unwrap();
    assert_eq!(session.raw(), "stage1 session/#1");
    assert_eq!(session.key(), "stage1-session-1");
}

#[test]
fn a_session_key_is_capped_at_96_chars() {
    let session = PlanSession::new("x".repeat(120)).unwrap();
    assert_eq!(session.key().len(), 96);
}

#[test]
fn a_blank_or_all_unsafe_session_is_rejected() {
    for raw in ["   ", "///", "#"] {
        assert_eq!(
            PlanSession::new(raw.to_string()).unwrap_err().to_string(),
            "Plan Approval challenge requires a nonblank session",
            "{raw:?}"
        );
    }
}

// --- PlanApprovalOperationId / PlanApprovalEventId -------------------------------

const V7: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000";

#[test]
fn the_approval_identifiers_accept_only_canonical_lowercase_uuid_v7() {
    let operation = PlanApprovalOperationId::parse(V7).unwrap();
    assert_eq!(operation.as_str(), V7);
    assert_eq!(operation.to_string(), V7);
    let event = PlanApprovalEventId::parse(V7).unwrap();
    assert_eq!(event.as_str(), V7);
    assert_eq!(event.to_string(), V7);
    for raw in [
        "not-a-uuid",
        "0190AAAA-BBBB-7CCC-9DDD-EEEEFFFF0000",
        "0190aaaa-bbbb-4ccc-9ddd-eeeeffff0000",
    ] {
        assert_eq!(
            PlanApprovalOperationId::parse(raw).unwrap_err().to_string(),
            "not a canonical UUIDv7 (expected lowercase 8-4-4-4-12)",
            "{raw}"
        );
        assert_eq!(
            PlanApprovalEventId::parse(raw).unwrap_err().to_string(),
            "not a canonical UUIDv7 (expected lowercase 8-4-4-4-12)",
            "{raw}"
        );
    }
}

#[test]
fn generated_approval_identifiers_round_trip_through_parse() {
    let operation = PlanApprovalOperationId::generate();
    assert_eq!(
        PlanApprovalOperationId::parse(operation.as_str()).unwrap(),
        operation
    );
    let event = PlanApprovalEventId::generate();
    assert_eq!(PlanApprovalEventId::parse(event.as_str()).unwrap(), event);
    assert_ne!(PlanApprovalEventId::generate(), event);
}

// --- PlanOfferedOptions ----------------------------------------------------------

#[test]
fn offered_options_keep_their_order_and_drop_blank_items() {
    let options = PlanOfferedOptions::parse(" Approve Plan , , Request Changes ,").unwrap();
    assert_eq!(
        options.values(),
        &["Approve Plan".to_string(), "Request Changes".to_string()]
    );
}

#[test]
fn offered_options_must_be_exactly_two() {
    for raw in ["", "Approve Plan", "a,b,c"] {
        assert_eq!(
            PlanOfferedOptions::parse(raw).unwrap_err().to_string(),
            "Plan Approval decision requires exactly two offered options",
            "{raw:?}"
        );
    }
}

// --- HookHealthTarget -------------------------------------------------------------

#[test]
fn a_hook_health_target_round_trips_its_relative_directory() {
    let before = HookHealthTarget::parse("spaces/default/intents").unwrap();
    assert_eq!(before.space().as_str(), "default");
    assert!(before.record().is_none());
    assert_eq!(before.relative_directory(), "spaces/default/intents");
    let after = HookHealthTarget::parse("spaces/default/intents/260911-fix").unwrap();
    assert_eq!(
        after.record().map(|record| record.as_str()),
        Some("260911-fix")
    );
    assert_eq!(
        after.relative_directory(),
        "spaces/default/intents/260911-fix"
    );
}

#[test]
fn a_hook_health_target_rejects_every_malformed_relative_directory() {
    for raw in [
        "intents/default",
        "spaces//intents",
        "spaces/default/records",
        "spaces/default/intents/260911-fix/extra",
        "spaces/default/intents/not a record",
    ] {
        assert_eq!(
            HookHealthTarget::parse(raw),
            Err(HookHealthError::InvalidIdentity),
            "{raw}"
        );
    }
}

// --- CodeGenerationAuthority::resolve ---------------------------------------------

const STATE: &str = "1111111111111111111111111111111111111111111111111111111111111111";

fn directive(published: PublishedDirective) -> ActiveDirective {
    ActiveDirective::new(
        3,
        IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap(),
        DirectivePublication::new(
            "2222222222222222222222222222222222222222222222222222222222222222".to_string(),
            STATE.to_string(),
            published,
        ),
        STATE.to_string(),
        "session".to_string(),
        1,
        1,
        3,
    )
}

fn run_stage(unit: Option<&str>) -> PublishedDirective {
    PublishedDirective::RunStage {
        stage: StageSlug::parse("code-generation").unwrap(),
        unit: unit.map(str::to_string),
    }
}

fn floor() -> CodeGenerationRunFloor {
    let at: DateTime<Utc> = DateTime::parse_from_rfc3339("2026-09-11T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    CodeGenerationRunFloor::new(1, 0, 0, 0, Some((RunBoundaryKind::WorkflowStarted, at))).unwrap()
}

#[test]
fn authority_requires_an_active_workflow_state() {
    let directive = directive(run_stage(None));
    assert_eq!(
        CodeGenerationAuthority::resolve(Some(&directive), Some(&floor()), None, None)
            .unwrap_err()
            .to_string(),
        "Code Generation approval authority requires an active workflow state"
    );
}

#[test]
fn authority_is_unavailable_without_a_current_directive_and_floor() {
    const UNAVAILABLE: &str = "Code Generation approval authority is unavailable because the active directive is missing, stale, or legacy; run a fresh `next`";
    let directive = directive(run_stage(None));
    // 指示が無い。
    assert_eq!(
        CodeGenerationAuthority::resolve(None, Some(&floor()), None, Some(STATE))
            .unwrap_err()
            .to_string(),
        UNAVAILABLE
    );
    // 状態が発行時と違う (stale)。
    assert_eq!(
        CodeGenerationAuthority::resolve(Some(&directive), Some(&floor()), None, Some("other"))
            .unwrap_err()
            .to_string(),
        UNAVAILABLE
    );
    // 実行境界が無い。
    assert_eq!(
        CodeGenerationAuthority::resolve(Some(&directive), None, None, Some(STATE))
            .unwrap_err()
            .to_string(),
        UNAVAILABLE
    );
}

#[test]
fn authority_rejects_a_directive_for_another_stage_or_kind() {
    let other_stage = directive(PublishedDirective::RunStage {
        stage: StageSlug::parse("build-and-test").unwrap(),
        unit: None,
    });
    assert_eq!(
        CodeGenerationAuthority::resolve(Some(&other_stage), Some(&floor()), None, Some(STATE))
            .unwrap_err()
            .to_string(),
        "Code Generation approval authority does not match active directive stage \"build-and-test\""
    );
    let steering = directive(PublishedDirective::LoadSteering {
        stage: StageSlug::parse("code-generation").unwrap(),
        part: 1,
        parts: 2,
        token: "t".to_string(),
    });
    assert_eq!(
        CodeGenerationAuthority::resolve(Some(&steering), Some(&floor()), None, Some(STATE))
            .unwrap_err()
            .to_string(),
        "Code Generation approval authority requires a run-stage or invoke-swarm directive, got \"load-steering\""
    );
    let error = directive(PublishedDirective::Error {
        stage: StageSlug::parse("code-generation").unwrap(),
    });
    assert_eq!(
        CodeGenerationAuthority::resolve(Some(&error), Some(&floor()), None, Some(STATE))
            .unwrap_err()
            .to_string(),
        "Code Generation approval authority requires a run-stage or invoke-swarm directive, got \"error\""
    );
}

#[test]
fn authority_binds_the_target_unit_to_the_issued_unit() {
    let stage_level = directive(run_stage(None));
    let per_unit = directive(run_stage(Some("u1")));
    assert_eq!(
        CodeGenerationAuthority::resolve(Some(&per_unit), Some(&floor()), Some("u2"), Some(STATE))
            .unwrap_err()
            .to_string(),
        "Code Generation approval target unit \"u2\" does not match active directive unit \"u1\""
    );
    assert_eq!(
        CodeGenerationAuthority::resolve(
            Some(&stage_level),
            Some(&floor()),
            Some("u1"),
            Some(STATE)
        )
        .unwrap_err()
        .to_string(),
        "Code Generation approval target unit \"u1\" does not match active directive unit \"(none)\""
    );
    assert_eq!(
        CodeGenerationAuthority::resolve(Some(&per_unit), Some(&floor()), None, Some(STATE))
            .unwrap_err()
            .to_string(),
        "Stage-level Code Generation approval requires a zero-Unit run-stage directive"
    );
    let resolved =
        CodeGenerationAuthority::resolve(Some(&per_unit), Some(&floor()), Some("u1"), Some(STATE))
            .unwrap();
    assert_eq!(resolved.target_id(), "unit:u1");
    assert_eq!(resolved.unit(), Some("u1"));
    assert_eq!(resolved.source_floor(), "unbindable");
    assert_eq!(
        resolved.run_floor(),
        "WORKFLOW_STARTED:2026-09-11T00:00:00Z#1"
    );
}

// --- PlanApprovalEvidence::verify_for_file ----------------------------------------

fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).unwrap().as_str().unwrap()
}

/// `plan-readiness.json` の `approved` 観測から、現在の契約が成立する材料を取り出す。
fn approved_case() -> (
    CodeGenerationAuthority,
    PlanApprovalDocuments,
    TestingPosture,
    String,
) {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../../../tests/golden/selfhost-stage1/plan-readiness.json"
    ))
    .unwrap();
    let case = corpus
        .get("observations")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|case| string(case, "id") == "approved")
        .unwrap();
    let value = case.get("authority").unwrap();
    let authority = CodeGenerationAuthority::new(
        &PlanTarget::stage_level(),
        &IntentId::parse(string(value, "intentId")).unwrap(),
        string(value, "directiveEpoch").to_string(),
        string(value, "runFloor").to_string(),
        string(value, "sourceFloor").to_string(),
        value.get("markerRevision").unwrap().as_u64().unwrap(),
    )
    .unwrap();
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
    )
    .unwrap();
    let source = case
        .get("current_source")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    (authority, documents, posture, source)
}

#[test]
fn the_approved_observation_verifies_against_its_own_answer() {
    let (authority, documents, posture, source) = approved_case();
    let answer = PlanQuestions::parse(documents.questions())
        .answer()
        .unwrap()
        .to_string();
    let evidence =
        PlanApprovalEvidence::verify(authority, &documents, &posture, Some(&source), &answer)
            .unwrap();
    assert_eq!(evidence.questions_file(), documents.questions_file());
}

#[test]
fn verification_rejects_a_changed_source_floor() {
    let (authority, documents, posture, _) = approved_case();
    assert_eq!(
        PlanApprovalEvidence::verify(authority, &documents, &posture, Some("other"), "")
            .unwrap_err()
            .to_string(),
        "Plan Approval requires workspace source to match the Code Generation directive's pre-planning source floor"
    );
}

#[test]
fn verification_rejects_a_questions_file_other_than_the_canonical_one() {
    let (authority, documents, posture, source) = approved_case();
    assert_eq!(
        PlanApprovalEvidence::verify_for_file(
            authority,
            &documents,
            Some(&posture),
            Some(&source),
            "elsewhere/questions.md",
            ""
        )
        .unwrap_err()
        .to_string(),
        format!(
            "Plan Approval questions file must be the active target's canonical file: {}",
            documents.questions_file()
        )
    );
}

#[test]
fn verification_rejects_empty_documents_and_a_stale_contract() {
    let (authority, documents, posture, source) = approved_case();
    let empty_plan = PlanApprovalDocuments::new(
        "  \n".to_string(),
        documents.instructions().to_string(),
        documents.questions().to_string(),
        documents.questions_file().to_string(),
    );
    assert_eq!(
        PlanApprovalEvidence::verify(authority.clone(), &empty_plan, &posture, Some(&source), "")
            .unwrap_err()
            .to_string(),
        "Plan Approval requires non-empty plan and unit-test instructions"
    );
    // 規則が解決できない (posture 無し) なら埋め込み契約は現在のものと言えない。
    assert_eq!(
        PlanApprovalEvidence::verify_for_file(
            authority,
            &documents,
            None,
            Some(&source),
            documents.questions_file(),
            ""
        )
        .unwrap_err()
        .to_string(),
        "Plan Approval requires the current Testing Contract"
    );
}

#[test]
fn verification_rejects_a_fingerprint_or_answer_that_does_not_match() {
    let (authority, documents, posture, source) = approved_case();
    let answer = PlanQuestions::parse(documents.questions())
        .answer()
        .unwrap()
        .to_string();
    // 計画が変われば指紋が変わり、質問ファイルの指紋と食い違う。
    let changed_plan = PlanApprovalDocuments::new(
        format!("{}\n\n- 追記\n", documents.plan()),
        documents.instructions().to_string(),
        documents.questions().to_string(),
        documents.questions_file().to_string(),
    );
    assert_eq!(
        PlanApprovalEvidence::verify(
            authority.clone(),
            &changed_plan,
            &posture,
            Some(&source),
            &answer
        )
        .unwrap_err()
        .to_string(),
        "Plan Approval fingerprint does not match the active intent, target, directive epoch, plan, instructions, and Testing Contract"
    );
    assert_eq!(
        PlanApprovalEvidence::verify(authority.clone(), &documents, &posture, Some(&source), "")
            .unwrap_err()
            .to_string(),
        "Plan Approval questions file must contain exactly [Answer]: (blank)"
    );
    assert_eq!(
        PlanApprovalEvidence::verify(authority, &documents, &posture, Some(&source), "Nope")
            .unwrap_err()
            .to_string(),
        "Plan Approval questions file must contain exactly [Answer]: Nope"
    );
}

// --- PlanApprovalRuntime::request_generation ---------------------------------------

/// 承認済みの判断でも、共有側が同じ受領を保持していなければ開始は許可されない。
#[test]
fn generation_requires_the_protected_receipt_to_be_held_by_the_shared_runtime() {
    use core_command_domain::orchestration::{
        CodeGenerationApproval, PlanApprovalOperationId, PlanApprovalReceipt, PlanApprovalRuntime,
        PlanDecisionEvidence, PlanGenerationStatus, PlanReceipts,
    };
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../../../tests/golden/selfhost-stage1/plan-readiness.json"
    ))
    .unwrap();
    let case = corpus
        .get("observations")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|case| string(case, "id") == "approved")
        .unwrap();
    let (authority, documents, posture, source) = approved_case();
    let value = case.get("receipt").unwrap();
    let evidence = PlanApprovalEvidence::new(
        authority.clone(),
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
        PlanGenerationStatus::Approved,
    )
    .unwrap();
    let approval = CodeGenerationApproval::evaluate(
        Ok(authority),
        &PlanTarget::stage_level(),
        &documents,
        Ok(posture),
        &PlanReceipts::new([receipt]).unwrap(),
        Some(&source),
    );
    assert!(approval.ok(), "{}", approval.reason());
    let at: DateTime<Utc> = "2026-09-11T00:00:00Z".parse().unwrap();
    let (mut runtime, _) = PlanApprovalRuntime::create(at);
    assert_eq!(
        runtime
            .request_generation(
                PlanApprovalOperationId::generate(),
                &approval,
                Some(&source),
                at
            )
            .unwrap_err()
            .to_string(),
        "Code Generation has no protected approval receipt"
    );
}
