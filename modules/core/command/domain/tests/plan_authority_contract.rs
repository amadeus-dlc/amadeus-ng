//! 固定本家が発行した計画承認challengeとの対応。
#![allow(clippy::unwrap_used)]
use base64::Engine as _;
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    ActiveDirective, CodeGenerationAuthority, CodeGenerationRunFloor, DirectivePublication,
    IntentId, PublishedDirective, RunBoundaryKind,
};
use core_command_domain::workflow_definition::StageSlug;
fn fixed_challenge() -> core_command_domain::orchestration::PlanChallenge {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests/golden/upstream-a277af21/stage1/cases.json"
    ))
    .unwrap();
    assert_eq!(
        corpus
            .get("source")
            .unwrap()
            .get("commit")
            .unwrap()
            .as_str(),
        Some("a277af218f0df7f325d3b8be7b6d90fce2c5bd40")
    );
    let observation = corpus
        .get("observations")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case.get("id").and_then(serde_json::Value::as_str) == Some("plan/decision"))
        .unwrap();
    let read_file = |collection: &str, suffix: &str| {
        let value = observation
            .get(collection)
            .unwrap()
            .as_object()
            .unwrap()
            .iter()
            .find(|(path, _)| path.ends_with(suffix))
            .unwrap()
            .1
            .as_str()
            .unwrap();
        base64::engine::general_purpose::STANDARD
            .decode(value)
            .unwrap()
    };
    let marker: serde_json::Value =
        serde_json::from_slice(&read_file("initial_files", ".aidlc-active-directive.json"))
            .unwrap();
    assert_eq!(marker.get("kind").unwrap().as_str(), Some("run-stage"));
    assert!(marker.get("unit").is_none());
    let challenge: serde_json::Value =
        serde_json::from_slice(&read_file("changed_files", "challenge-stage1-session.json"))
            .unwrap();
    let text = |name: &str| marker.get(name).unwrap().as_str().unwrap().to_string();
    let publication = DirectivePublication::new(
        text("project_sha256"),
        text("state_sha256"),
        PublishedDirective::RunStage {
            stage: StageSlug::parse("code-generation").unwrap(),
            unit: None,
        },
    )
    .with_source_floor(Some(text("code_generation_source_sha256")));
    let directive = ActiveDirective::new(
        marker.get("revision").unwrap().as_u64().unwrap(),
        IntentId::parse(&text("intent_uuid")).unwrap(),
        publication,
        text("state_sha256"),
        text("owner_session"),
        marker.get("owner_epoch").unwrap().as_u64().unwrap(),
        marker.get("context_epoch").unwrap().as_u64().unwrap(),
        marker.get("revision").unwrap().as_u64().unwrap(),
    );
    let floor_text = challenge.get("runFloor").unwrap().as_str().unwrap();
    let (timestamp, ordinal) = floor_text
        .strip_prefix("WORKFLOW_STARTED:")
        .unwrap()
        .split_once('#')
        .unwrap();
    assert_eq!(ordinal, "1");
    let at = DateTime::parse_from_rfc3339(timestamp)
        .unwrap()
        .with_timezone(&Utc);
    let floor =
        CodeGenerationRunFloor::new(1, 0, 0, 0, Some((RunBoundaryKind::WorkflowStarted, at)))
            .unwrap();
    let authority = CodeGenerationAuthority::resolve(
        Some(&directive),
        Some(&floor),
        None,
        Some(&text("state_sha256")),
    );
    assert!(authority.is_ok(), "{authority:?}");
    let authority = authority.unwrap();
    assert_eq!(
        authority.directive_epoch(),
        challenge.get("directiveEpoch").unwrap().as_str().unwrap()
    );
    let plan = String::from_utf8(read_file("initial_files", "/code-generation-plan.md")).unwrap();
    let instructions =
        String::from_utf8(read_file("initial_files", "/unit-test-instructions.md")).unwrap();
    let embedded = plan
        .split("```json\n")
        .nth(1)
        .unwrap()
        .split("\n```")
        .next()
        .unwrap();
    let contract: serde_json::Value = serde_json::from_str(embedded).unwrap();
    let fingerprint = authority.approval_fingerprint(
        &plan,
        &instructions,
        contract.get("contract_sha256").unwrap().as_str().unwrap(),
    );
    assert_eq!(
        fingerprint,
        challenge.get("fingerprint").unwrap().as_str().unwrap()
    );
    let questions =
        String::from_utf8(read_file("initial_files", "/code-generation-questions.md")).unwrap();
    assert_eq!(
        core_command_domain::orchestration::PlanQuestions::parse(&questions).answer(),
        Some("")
    );
    use core_command_domain::orchestration::{
        PlanApprovalDocuments, PlanApprovalEvidence, TestingContext, TestingPosture,
        TestingSections,
    };
    let references: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests/golden/selfhost-stage1/bugfix-first-next.json"
    ))
    .unwrap();
    assert_eq!(references.get("source"), corpus.get("source"));
    let memory = |name: &str| {
        let encoded = references
            .get("reference_files")
            .unwrap()
            .get(format!("aidlc/spaces/default/memory/{name}.md"))
            .unwrap()
            .as_str()
            .unwrap();
        String::from_utf8(
            base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .unwrap(),
        )
        .unwrap()
    };
    let posture = TestingPosture::resolve(
        &TestingSections::from_documents(&memory("org"), &memory("team"), &memory("project")),
        &TestingContext::new(
            "bugfix".to_string(),
            "minimal".to_string(),
            "brownfield".to_string(),
        ),
    )
    .unwrap();
    let documents = PlanApprovalDocuments::new(
        plan,
        instructions,
        questions,
        challenge
            .get("questionsFile")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string(),
    );
    let source = authority.source_floor().to_string();
    let evidence = PlanApprovalEvidence::verify(authority, &documents, &posture, Some(&source), "");
    assert!(evidence.is_ok(), "{evidence:?}");
    let evidence = evidence.unwrap();
    assert_eq!(
        evidence.prompt_sha256(),
        challenge.get("promptSha256").unwrap().as_str().unwrap()
    );
    use core_command_domain::orchestration::{PlanChallenge, PlanSession};
    let offered = PlanChallenge::issue(
        evidence,
        PlanSession::new("stage1-session".to_string()).unwrap(),
        ["Approve Plan".to_string(), "Request Changes".to_string()],
        false,
    );
    assert_eq!(
        offered.id(),
        challenge.get("challengeId").unwrap().as_str().unwrap()
    );
    let choices: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests/golden/selfhost-stage1/plan-choices.json"
    ))
    .unwrap();
    assert_eq!(choices.get("source"), corpus.get("source"));
    for case in choices.get("observations").unwrap().as_array().unwrap() {
        let options = case.get("options").unwrap().as_array().unwrap();
        let candidate = PlanChallenge::issue(
            offered.evidence().clone(),
            PlanSession::new(case.get("session").unwrap().as_str().unwrap().to_string()).unwrap(),
            [
                options.first().unwrap().as_str().unwrap().to_string(),
                options.get(1).unwrap().as_str().unwrap().to_string(),
            ],
            case.get("exact").unwrap().as_bool().unwrap(),
        );
        assert_eq!(
            candidate.id(),
            case.get("challenge_id").unwrap().as_str().unwrap()
        );
        let choice = candidate.offered_choice(case.get("response").unwrap().as_str().unwrap());
        assert_eq!(
            choice.map(core_command_domain::orchestration::PlanChoice::as_str),
            case.get("choice").unwrap().as_str(),
            "{}",
            case.get("id").unwrap()
        );
    }
    offered
}

#[test]
fn the_directive_epoch_matches_the_fixed_upstream_challenge() {
    let _ = fixed_challenge();
}

#[test]
fn a_shared_runtime_issues_the_exact_challenge_for_its_session() {
    use core_command_domain::orchestration::{
        PlanApprovalEvent, PlanApprovalOperationId, PlanApprovalRuntime, PlanSession,
    };
    let offered = fixed_challenge();
    let at = DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let (mut runtime, genesis) = PlanApprovalRuntime::create(at);
    assert!(matches!(genesis, PlanApprovalEvent::Created(_)));
    let id = PlanApprovalOperationId::generate();
    let issued = runtime
        .issue_challenge(id.clone(), offered.clone(), at)
        .unwrap();
    assert!(matches!(issued, PlanApprovalEvent::ChallengeIssued(_)));
    let current = runtime.challenges().for_session(offered.session()).unwrap();
    assert_eq!(current.id(), &id);
    assert_eq!(current.challenge(), &offered);
    assert_eq!(runtime.seq_nr(), 2);
    assert!(
        runtime
            .challenges()
            .for_session(&PlanSession::new("other".to_string()).unwrap())
            .is_none()
    );
}

#[test]
fn a_human_response_is_bound_to_the_observed_issuance() {
    use core_command_domain::orchestration::{
        PlanApprovalOperationId, PlanApprovalRuntime, PlanChoice,
    };
    let challenge = fixed_challenge();
    let at = DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let (mut runtime, _) = PlanApprovalRuntime::create(at);
    let issuance = PlanApprovalOperationId::generate();
    runtime
        .issue_challenge(issuance.clone(), challenge.clone(), at)
        .unwrap();
    let observation = PlanApprovalOperationId::generate();
    runtime
        .observe_response(observation.clone(), &issuance, challenge.session(), "1", at)
        .unwrap();
    let response = runtime
        .challenges()
        .for_session(challenge.session())
        .unwrap()
        .response()
        .unwrap();
    assert_eq!(response.id(), &observation);
    assert_eq!(response.occurrence_id(), &issuance);
    assert_eq!(response.choice(), PlanChoice::ApprovePlan);
    assert_eq!(
        response.response_sha256(),
        core_infrastructure::hash::sha256_hex(b"1")
    );
}

#[test]
fn reissuing_identical_questions_never_rebinds_an_old_observation() {
    use core_command_domain::orchestration::{PlanApprovalOperationId, PlanApprovalRuntime};
    let challenge = fixed_challenge();
    let at = DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let (mut runtime, _) = PlanApprovalRuntime::create(at);
    let base = runtime.clone();
    let old = PlanApprovalOperationId::generate();
    let first = runtime
        .issue_challenge(old.clone(), challenge.clone(), at)
        .unwrap();
    let observed = runtime
        .observe_response(
            PlanApprovalOperationId::generate(),
            &old,
            challenge.session(),
            "1",
            at,
        )
        .unwrap();
    let replacement = PlanApprovalOperationId::generate();
    let issued_again = runtime
        .issue_challenge(replacement.clone(), challenge.clone(), at)
        .unwrap();
    assert!(
        runtime
            .challenges()
            .for_session(challenge.session())
            .unwrap()
            .response()
            .is_none()
    );
    let late = runtime
        .observe_response(
            PlanApprovalOperationId::generate(),
            &old,
            challenge.session(),
            "1",
            at,
        )
        .unwrap();
    let current = runtime
        .challenges()
        .for_session(challenge.session())
        .unwrap();
    assert_eq!(current.id(), &replacement);
    assert_eq!(current.challenge().id(), challenge.id());
    assert!(
        current.response().is_none(),
        "内容由来のchallengeIdが同じでも古い発行回の応答は使わない"
    );
    let replayed = PlanApprovalRuntime::replay(
        base,
        [
            (first, 2, at),
            (observed, 3, at),
            (issued_again, 4, at),
            (late, 5, at),
        ],
    );
    assert_eq!(replayed, runtime);
}

#[test]
fn session_key_collisions_and_unrelated_turns_cannot_replace_a_captured_response() {
    use core_command_domain::orchestration::{
        PlanApprovalOperationId, PlanApprovalRuntime, PlanChallenge, PlanSession,
    };
    let source = fixed_challenge();
    let session = PlanSession::new("session/a".to_string()).unwrap();
    let alias = PlanSession::new("session?a".to_string()).unwrap();
    assert_eq!(session.key(), alias.key());
    let challenge = PlanChallenge::issue(
        source.evidence().clone(),
        session.clone(),
        source.options().clone(),
        false,
    );
    let at = DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let (mut runtime, _) = PlanApprovalRuntime::create(at);
    let issuance = PlanApprovalOperationId::generate();
    runtime
        .issue_challenge(issuance.clone(), challenge, at)
        .unwrap();
    runtime
        .observe_response(
            PlanApprovalOperationId::generate(),
            &issuance,
            &alias,
            "1",
            at,
        )
        .unwrap();
    assert!(
        runtime
            .challenges()
            .for_session(&session)
            .unwrap()
            .response()
            .is_none()
    );
    runtime
        .observe_response(
            PlanApprovalOperationId::generate(),
            &issuance,
            &session,
            "1",
            at,
        )
        .unwrap();
    let captured = runtime
        .challenges()
        .for_session(&session)
        .unwrap()
        .response()
        .cloned();
    for (name, text) in [
        (&alias, "2"),
        (&session, "continue"),
        (&session, "\u{0085}2"),
    ] {
        runtime
            .observe_response(
                PlanApprovalOperationId::generate(),
                &issuance,
                name,
                text,
                at,
            )
            .unwrap();
        assert_eq!(
            runtime
                .challenges()
                .for_session(&session)
                .unwrap()
                .response()
                .cloned(),
            captured
        );
    }
}

#[test]
fn preparing_a_publication_blocks_approval_until_its_result_is_known() {
    use core_command_domain::orchestration::{
        IntentExecutionId, PlanApprovalOperationId, PlanApprovalRuntime, PlanInvalidation,
    };
    use core_command_domain::workspace::SpaceName;
    let challenge = fixed_challenge();
    let at = DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let (mut runtime, _) = PlanApprovalRuntime::create(at);
    let issuance = PlanApprovalOperationId::generate();
    runtime
        .issue_challenge(issuance.clone(), challenge.clone(), at)
        .unwrap();
    let invalidation = PlanInvalidation::new(
        PlanApprovalOperationId::generate(),
        SpaceName::parse("other-space").unwrap(),
        IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
    );
    runtime
        .prepare_invalidation(invalidation.clone(), at)
        .unwrap();
    assert_eq!(
        runtime.invalidations().iter().collect::<Vec<_>>(),
        vec![&invalidation]
    );
    let before = runtime.clone();
    assert!(
        runtime
            .issue_challenge(PlanApprovalOperationId::generate(), challenge.clone(), at)
            .is_err()
    );
    assert!(
        runtime
            .observe_response(
                PlanApprovalOperationId::generate(),
                &issuance,
                challenge.session(),
                "1",
                at
            )
            .is_err()
    );
    assert_eq!(runtime, before, "未完了の発行を越えて承認状態を変更しない");
}

#[test]
fn resolving_a_failed_publication_preserves_the_offer_but_a_committed_one_invalidates_it() {
    use core_command_domain::orchestration::{
        IntentExecutionId, PlanApprovalOperationId, PlanApprovalRuntime, PlanInvalidation,
    };
    use core_command_domain::workspace::SpaceName;
    let challenge = fixed_challenge();
    let at = DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    for published in [false, true] {
        let (mut runtime, _) = PlanApprovalRuntime::create(at);
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
        let before = runtime.challenges().clone();
        let operation = PlanApprovalOperationId::generate();
        let prepared = runtime
            .prepare_invalidation(
                PlanInvalidation::new(
                    operation.clone(),
                    SpaceName::parse("different-space").unwrap(),
                    IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
                ),
                at,
            )
            .unwrap();
        let base = runtime.clone();
        let (space, source) =
            approval_publication::source_execution(&runtime, &operation, published, at);
        let event = runtime
            .resolve_for_publication(&operation, &space, &source, at)
            .unwrap();
        assert!(runtime.invalidations().is_empty());
        if published {
            assert!(
                runtime
                    .challenges()
                    .for_session(challenge.session())
                    .is_none()
            );
        } else {
            assert_eq!(runtime.challenges(), &before);
        }
        assert_eq!(PlanApprovalRuntime::replay(base, [(event, 5, at)]), runtime);
        assert_ne!(
            prepared.id().as_str(),
            runtime.id().to_string(),
            "イベントIDを集約IDに流用しない"
        );
    }
}

#[test]
fn retrying_an_old_operation_cannot_reissue_or_overwrite_a_response() {
    use core_command_domain::orchestration::{
        IntentExecutionId, PlanApprovalOperationId, PlanApprovalRuntime, PlanInvalidation,
    };
    use core_command_domain::workspace::SpaceName;
    let challenge = fixed_challenge();
    let at = DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let (mut runtime, _) = PlanApprovalRuntime::create(at);
    let issuance = PlanApprovalOperationId::generate();
    runtime
        .issue_challenge(issuance.clone(), challenge.clone(), at)
        .unwrap();
    let response = PlanApprovalOperationId::generate();
    runtime
        .observe_response(response.clone(), &issuance, challenge.session(), "1", at)
        .unwrap();
    let before = runtime.clone();
    assert!(
        runtime
            .issue_challenge(issuance.clone(), challenge.clone(), at)
            .is_err()
    );
    assert!(
        runtime
            .observe_response(response.clone(), &issuance, challenge.session(), "2", at)
            .is_err()
    );
    assert_eq!(runtime, before);
    let operation = PlanApprovalOperationId::generate();
    let preparation = PlanInvalidation::new(
        operation.clone(),
        SpaceName::parse("default").unwrap(),
        IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
    );
    runtime
        .prepare_invalidation(preparation.clone(), at)
        .unwrap();
    let pending = runtime.clone();
    assert!(
        runtime
            .prepare_invalidation(preparation.clone(), at)
            .is_err()
    );
    assert_eq!(runtime, pending);
    let (space, source) = approval_publication::source_execution(&runtime, &operation, true, at);
    runtime
        .resolve_for_publication(&operation, &space, &source, at)
        .unwrap();
    let invalidated = runtime.clone();
    assert!(
        runtime
            .issue_challenge(issuance.clone(), challenge.clone(), at)
            .is_err()
    );
    assert!(
        runtime
            .observe_response(response, &issuance, challenge.session(), "1", at)
            .is_err()
    );
    assert!(runtime.prepare_invalidation(preparation, at).is_err());
    assert!(
        runtime
            .resolve_for_publication(&operation, &space, &source, at)
            .is_err()
    );
    assert_eq!(runtime, invalidated, "失効後も過去の操作は再実行しない");
}

#[test]
fn recorded_challenge_values_reconstruct_without_reinterpreting_current_documents() {
    use core_command_domain::orchestration::{
        CodeGenerationAuthority, PlanApprovalEvidence, PlanChallenge, PlanTarget,
    };
    let challenge = fixed_challenge();
    let source = challenge.evidence();
    let old = source.authority();
    let authority = CodeGenerationAuthority::new(
        &PlanTarget::stage_level(),
        &IntentId::parse(old.intent_id()).unwrap(),
        old.directive_epoch().to_string(),
        old.run_floor().to_string(),
        old.source_floor().to_string(),
        old.marker_revision(),
    )
    .unwrap();
    assert_eq!(&authority, old);
    let evidence = PlanApprovalEvidence::new(
        authority,
        source.fingerprint().to_string(),
        source.questions_file().to_string(),
        source.questions_sha256().to_string(),
        source.prompt_sha256().to_string(),
    )
    .unwrap();
    assert_eq!(&evidence, source);
    let reconstructed = PlanChallenge::issue(
        evidence,
        challenge.session().clone(),
        challenge.options().clone(),
        challenge.require_exact(),
    );
    assert_eq!(reconstructed, challenge);
}

#[test]
fn recorded_approval_values_reject_invalid_fingerprints_and_paths() {
    use core_command_domain::orchestration::{
        CodeGenerationAuthority, PlanApprovalEvidence, PlanTarget,
    };
    let challenge = fixed_challenge();
    let evidence = challenge.evidence();
    let old = evidence.authority();
    for (epoch, floor, source) in [
        ("sha256:short", old.run_floor(), old.source_floor()),
        (old.directive_epoch(), "invented", old.source_floor()),
        (
            old.directive_epoch(),
            "WORKFLOW_STARTED:invalid#1",
            old.source_floor(),
        ),
        (
            old.directive_epoch(),
            "WORKFLOW_STARTED:2026-09-08T01:00:00Z#0",
            old.source_floor(),
        ),
        (old.directive_epoch(), old.run_floor(), "../source"),
    ] {
        assert!(
            CodeGenerationAuthority::new(
                &PlanTarget::stage_level(),
                &IntentId::parse(old.intent_id()).unwrap(),
                epoch.to_string(),
                floor.to_string(),
                source.to_string(),
                old.marker_revision()
            )
            .is_err(),
            "{epoch} {floor} {source}"
        );
    }
    for (fingerprint, file, question, prompt) in [
        (
            "sha256:short",
            evidence.questions_file(),
            evidence.questions_sha256(),
            evidence.prompt_sha256(),
        ),
        (
            evidence.fingerprint(),
            "../questions.md",
            evidence.questions_sha256(),
            evidence.prompt_sha256(),
        ),
        (
            evidence.fingerprint(),
            "/questions.md",
            evidence.questions_sha256(),
            evidence.prompt_sha256(),
        ),
        (
            evidence.fingerprint(),
            "aidlc/../questions.md",
            evidence.questions_sha256(),
            evidence.prompt_sha256(),
        ),
        (
            evidence.fingerprint(),
            "aidlc\\questions.md",
            evidence.questions_sha256(),
            evidence.prompt_sha256(),
        ),
        (
            evidence.fingerprint(),
            evidence.questions_file(),
            "short",
            evidence.prompt_sha256(),
        ),
        (
            evidence.fingerprint(),
            evidence.questions_file(),
            evidence.questions_sha256(),
            "short",
        ),
    ] {
        assert!(
            PlanApprovalEvidence::new(
                old.clone(),
                fingerprint.to_string(),
                file.to_string(),
                question.to_string(),
                prompt.to_string()
            )
            .is_err(),
            "{fingerprint} {file} {question} {prompt}"
        );
    }
}

#[test]
fn a_runtime_snapshot_retains_pending_invalidation_and_the_observed_issuance() {
    use core_command_domain::orchestration::{
        IntentExecutionId, PlanApprovalOperationId, PlanApprovalRuntime, PlanInvalidation,
    };
    use core_command_domain::workspace::SpaceName;
    let challenge = fixed_challenge();
    let at = DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let (mut runtime, _) = PlanApprovalRuntime::create(at);
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
    let operation = PlanApprovalOperationId::generate();
    runtime
        .prepare_invalidation(
            PlanInvalidation::new(
                operation,
                SpaceName::parse("default").unwrap(),
                IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
            ),
            at,
        )
        .unwrap();
    runtime = runtime.with_version(9);
    let reconstructed = PlanApprovalRuntime::new(
        core_command_domain::orchestration::PlanGenerations::default(),
        core_command_domain::orchestration::PlanAnswers::default(),
        core_command_domain::orchestration::PlanReceipts::default(),
        core_command_domain::orchestration::PlanPendingResponses::default(),
        *runtime.id(),
        runtime.challenges().clone(),
        runtime.invalidations().clone(),
        runtime.applied_operations().clone(),
        runtime.seq_nr(),
        runtime.last_updated_at(),
    )
    .unwrap()
    .with_version(runtime.version());
    assert_eq!(runtime, reconstructed);
    assert!(
        PlanApprovalRuntime::new(
            core_command_domain::orchestration::PlanGenerations::default(),
            core_command_domain::orchestration::PlanAnswers::default(),
            core_command_domain::orchestration::PlanReceipts::default(),
            core_command_domain::orchestration::PlanPendingResponses::default(),
            *runtime.id(),
            runtime.challenges().clone(),
            runtime.invalidations().clone(),
            runtime.applied_operations().clone(),
            0,
            runtime.last_updated_at()
        )
        .is_err()
    );
}

#[test]
fn a_recorded_human_response_requires_a_valid_digest() {
    use core_command_domain::orchestration::{
        PlanApprovalOperationId, PlanChoice, PlanHumanResponse,
    };
    let observation = PlanApprovalOperationId::generate();
    let issuance = PlanApprovalOperationId::generate();
    for digest in [
        "".to_string(),
        "x".repeat(64),
        "A".repeat(64),
        "a".repeat(63),
        "a".repeat(65),
    ] {
        assert!(
            PlanHumanResponse::new(
                observation.clone(),
                issuance.clone(),
                PlanChoice::ApprovePlan,
                digest
            )
            .is_err()
        );
    }
    assert!(
        PlanHumanResponse::new(
            observation,
            issuance,
            PlanChoice::ApprovePlan,
            "a".repeat(64)
        )
        .is_ok()
    );
}

#[test]
#[should_panic(expected = "approval operation already applied")]
fn replay_rejects_a_second_event_reusing_an_applied_operation() {
    use core_command_domain::orchestration::{PlanApprovalOperationId, PlanApprovalRuntime};
    let at = DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let (mut runtime, _) = PlanApprovalRuntime::create(at);
    let event = runtime
        .issue_challenge(PlanApprovalOperationId::generate(), fixed_challenge(), at)
        .unwrap();
    runtime.apply_event(&event, 3, at);
}

#[path = "../../../../../tests/support/approval_publication.rs"]
mod approval_publication;

#[test]
fn preparing_a_human_response_pins_the_offered_issuance_before_other_writes() {
    use core_command_domain::orchestration::{
        IntentExecutionId, PlanApprovalOperationId, PlanApprovalOrigin, PlanApprovalRuntime,
        PlanChoice,
    };
    use core_command_domain::workspace::SpaceName;
    let challenge = fixed_challenge();
    let at = DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let (mut runtime, _) = PlanApprovalRuntime::create(at);
    let offered = PlanApprovalOperationId::generate();
    runtime
        .issue_challenge(offered.clone(), challenge.clone(), at)
        .unwrap();
    let observation = PlanApprovalOperationId::generate();
    let origin = PlanApprovalOrigin::new(
        SpaceName::default(),
        IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
    );
    runtime
        .prepare_response(
            observation.clone(),
            origin.clone(),
            challenge.session().clone(),
            "1",
            at,
        )
        .unwrap();
    let pending = runtime.pending_responses().get(&observation).unwrap();
    assert_eq!(pending.origin(), &origin);
    assert_eq!(pending.occurrence_id(), &offered);
    assert_eq!(pending.response(), "1");
    assert_eq!(pending.choice(), Some(PlanChoice::ApprovePlan));
    assert!(
        runtime
            .challenges()
            .for_session(challenge.session())
            .unwrap()
            .response()
            .is_none(),
        "準備だけでは応答を受領済みにしない"
    );
    assert!(
        runtime
            .issue_challenge(PlanApprovalOperationId::generate(), challenge, at)
            .is_err(),
        "未完了の応答を回復してから新しい質問へ進む"
    );
}

#[test]
fn a_prepared_response_is_delivered_once_then_bound_to_its_original_offer() {
    use core_command_domain::orchestration::{
        IntentExecutionId, PlanApprovalOperationId, PlanApprovalOrigin, PlanApprovalRuntime,
        PlanChoice, PlanResponseDelivery,
    };
    use core_command_domain::workspace::SpaceName;
    let challenge = fixed_challenge();
    let at = DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let (mut runtime, _) = PlanApprovalRuntime::create(at);
    let offered = PlanApprovalOperationId::generate();
    runtime
        .issue_challenge(offered.clone(), challenge.clone(), at)
        .unwrap();
    let id = PlanApprovalOperationId::generate();
    runtime
        .prepare_response(
            id.clone(),
            PlanApprovalOrigin::new(
                SpaceName::default(),
                IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
            ),
            challenge.session().clone(),
            "1",
            at,
        )
        .unwrap();
    let mut source = approval_response::response_execution(&runtime, &id, at);
    assert_eq!(
        runtime
            .response_delivery(&id, &SpaceName::default(), &source)
            .unwrap(),
        PlanResponseDelivery::RecordRequired
    );
    assert!(
        runtime
            .complete_response(&id, &SpaceName::default(), &source, at)
            .is_err()
    );
    source.record_prepared_response(&runtime, &id, at).unwrap();
    assert!(source.has_approval_observation(&id));
    assert_eq!(
        runtime
            .response_delivery(&id, &SpaceName::default(), &source)
            .unwrap(),
        PlanResponseDelivery::Recorded
    );
    let before = source.clone();
    assert!(source.record_prepared_response(&runtime, &id, at).is_err());
    assert_eq!(source, before);
    source
        .observe_prompt("unrelated", "continue", false, at)
        .unwrap();
    assert!(
        source.has_approval_observation(&id),
        "最新の人間応答だけを根拠に二重配信しない"
    );
    runtime
        .complete_response(&id, &SpaceName::default(), &source, at)
        .unwrap();
    assert!(runtime.pending_responses().is_empty());
    let response = runtime
        .challenges()
        .for_session(challenge.session())
        .unwrap()
        .response()
        .unwrap();
    assert_eq!(response.id(), &id);
    assert_eq!(response.occurrence_id(), &offered);
    assert_eq!(response.choice(), PlanChoice::ApprovePlan);
}

#[path = "../../../../../tests/support/approval_response.rs"]
mod approval_response;

#[test]
fn a_plan_answer_requires_the_actual_response_and_binds_the_answered_document() {
    use core_command_domain::orchestration::{
        IntentExecutionId, PlanAnswerInput, PlanAnswerState, PlanApprovalEvidence,
        PlanApprovalOperationId, PlanApprovalOrigin, PlanApprovalRuntime, PlanChoice,
        PlanDecisionEvidence, PlanGenerationStatus,
    };
    use core_command_domain::workspace::SpaceName;
    let challenge = fixed_challenge();
    let at = DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let (mut runtime, _) = PlanApprovalRuntime::create(at);
    let offered = PlanApprovalOperationId::generate();
    runtime
        .issue_challenge(offered.clone(), challenge.clone(), at)
        .unwrap();
    let evidence = challenge.evidence();
    let answered = PlanApprovalEvidence::new(
        evidence.authority().clone(),
        evidence.fingerprint().to_string(),
        evidence.questions_file().to_string(),
        "f".repeat(64),
        evidence.prompt_sha256().to_string(),
    )
    .unwrap();
    let input = PlanAnswerInput::new(
        PlanApprovalOrigin::new(
            SpaceName::default(),
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
        ),
        "code-generation".to_string(),
        PlanDecisionEvidence::new(answered.clone(), challenge.session().clone()),
        PlanChoice::ApprovePlan,
        Some(evidence.authority().source_floor().to_string()),
    );
    let before = runtime.clone();
    assert!(
        runtime
            .record_answer(PlanApprovalOperationId::generate(), input.clone(), at)
            .is_err()
    );
    assert_eq!(runtime, before);
    let response = PlanApprovalOperationId::generate();
    runtime
        .observe_response(response.clone(), &offered, challenge.session(), "1", at)
        .unwrap();
    let id = PlanApprovalOperationId::generate();
    runtime.record_answer(id.clone(), input, at).unwrap();
    let answer = runtime.answers().get(&id).unwrap();
    assert_eq!(answer.state(), &PlanAnswerState::Pending);
    assert_eq!(answer.response_id(), &response);
    assert_eq!(answer.occurrence_id(), &offered);
    let receipt = answer.receipt().unwrap();
    assert_eq!(receipt.decision().evidence(), &answered);
    assert_eq!(receipt.status(), PlanGenerationStatus::Approved);
    assert_eq!(runtime.receipts().get(&receipt.key()), Some(receipt));
    let inconsistent = PlanApprovalRuntime::new(
        core_command_domain::orchestration::PlanGenerations::default(),
        runtime.answers().clone(),
        core_command_domain::orchestration::PlanReceipts::default(),
        runtime.pending_responses().clone(),
        *runtime.id(),
        runtime.challenges().clone(),
        runtime.invalidations().clone(),
        runtime.applied_operations().clone(),
        runtime.seq_nr(),
        at,
    );
    assert!(
        inconsistent.is_err(),
        "監査待ちの承認に受領候補がない復元状態を拒否する"
    );
    let orphan = PlanApprovalRuntime::new(
        core_command_domain::orchestration::PlanGenerations::default(),
        core_command_domain::orchestration::PlanAnswers::default(),
        runtime.receipts().clone(),
        runtime.pending_responses().clone(),
        *runtime.id(),
        runtime.challenges().clone(),
        runtime.invalidations().clone(),
        runtime.applied_operations().clone(),
        runtime.seq_nr(),
        at,
    );
    assert!(orphan.is_err(), "回答の保存事実がない受領を復元しない");
    let source_hash = receipt.certified_source().to_string();
    let receipt_key = receipt.key();
    let mut guarded = runtime.clone();
    assert!(
        guarded
            .issue_challenge(PlanApprovalOperationId::generate(), challenge.clone(), at)
            .is_err()
    );
    assert!(
        guarded
            .prepare_invalidation(
                core_command_domain::orchestration::PlanInvalidation::new(
                    PlanApprovalOperationId::generate(),
                    SpaceName::default(),
                    answer.input().origin().execution_id().clone()
                ),
                at
            )
            .is_err()
    );
    assert!(
        guarded
            .prepare_response(
                PlanApprovalOperationId::generate(),
                answer.input().origin().clone(),
                challenge.session().clone(),
                "1",
                at
            )
            .is_err()
    );
    assert_eq!(guarded, runtime);

    let source_before_audit = approval_answer::source_execution(&runtime, &id, at);
    let mut rejected = runtime.clone();
    rejected
        .abort_answer(&id, &SpaceName::default(), &source_before_audit, None, at)
        .unwrap();
    assert!(rejected.receipts().get(&receipt_key).is_none());
    assert!(
        matches!(rejected.answers().get(&id).unwrap().state(), PlanAnswerState::Aborted(message) if message == "Plan Approval source changed during receipt certification; present the current plan again")
    );
    assert_eq!(rejected.challenges(), runtime.challenges());
    assert!(
        rejected
            .abort_answer(&id, &SpaceName::default(), &source_before_audit, None, at)
            .is_err()
    );

    let mut source = approval_answer::source_execution(&runtime, &id, at);
    use core_command_domain::orchestration::PlanAnswerDelivery;
    assert_eq!(
        runtime
            .answer_delivery(&id, &SpaceName::default(), &source, Some(&source_hash))
            .unwrap(),
        PlanAnswerDelivery::RecordRequired
    );
    assert_eq!(
        runtime
            .answer_delivery(&id, &SpaceName::default(), &source, None)
            .unwrap(),
        PlanAnswerDelivery::RejectCertification
    );
    assert!(
        runtime
            .complete_answer(&id, &SpaceName::default(), &source, at)
            .is_err()
    );
    source
        .record_plan_answer(&runtime, &id, &SpaceName::default(), Some(&source_hash), at)
        .unwrap();
    assert!(source.has_plan_answer(&id));
    assert!(
        source
            .record_plan_answer(&runtime, &id, &SpaceName::default(), Some(&source_hash), at)
            .is_err()
    );
    runtime
        .complete_answer(&id, &SpaceName::default(), &source, at)
        .unwrap();
    assert_eq!(
        runtime.answers().get(&id).unwrap().state(),
        &PlanAnswerState::Recorded
    );
    assert!(
        runtime
            .challenges()
            .for_session(challenge.session())
            .is_none()
    );
}

/// 保留中の操作が他の操作を拒む理由は `PlanRuntimeError` の変種で名指しされる。
///
/// 上の契約は `is_err()` で拒否だけを見ている。ここでは各ガードが**どの理由**で拒むかを
/// 固定する — 出す側は変種ごとに逐語文言を選ぶので、変種の取り違えは文言の取り違えになる。
#[test]
fn pending_operations_name_the_exact_reason_they_block_other_operations() {
    use core_command_domain::orchestration::{
        IntentExecutionId, PlanAnswerInput, PlanApprovalEvidence, PlanApprovalOperationId,
        PlanApprovalOrigin, PlanApprovalRuntime, PlanChoice, PlanDecisionEvidence,
        PlanInvalidation, PlanRuntimeError,
    };
    use core_command_domain::workspace::SpaceName;
    let challenge = fixed_challenge();
    let at = DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let execution_id = IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap();
    let origin = PlanApprovalOrigin::new(SpaceName::default(), execution_id.clone());
    let other_space = SpaceName::parse("other").unwrap();
    let (mut runtime, _) = PlanApprovalRuntime::create(at);
    let offered = PlanApprovalOperationId::generate();
    runtime
        .issue_challenge(offered.clone(), challenge.clone(), at)
        .unwrap();

    // 応答の準備が保留中: 次の準備・失効準備・観測はすべて `PendingResponse`。
    let mut with_response = runtime.clone();
    let prepared = PlanApprovalOperationId::generate();
    with_response
        .prepare_response(
            prepared.clone(),
            origin.clone(),
            challenge.session().clone(),
            "1",
            at,
        )
        .unwrap();
    assert_eq!(
        with_response
            .prepare_response(
                PlanApprovalOperationId::generate(),
                origin.clone(),
                challenge.session().clone(),
                "1",
                at,
            )
            .unwrap_err(),
        PlanRuntimeError::PendingResponse
    );
    assert_eq!(
        with_response
            .prepare_invalidation(
                PlanInvalidation::new(
                    PlanApprovalOperationId::generate(),
                    SpaceName::default(),
                    execution_id.clone(),
                ),
                at,
            )
            .unwrap_err(),
        PlanRuntimeError::PendingResponse
    );
    assert_eq!(
        with_response
            .observe_response(
                PlanApprovalOperationId::generate(),
                &offered,
                challenge.session(),
                "1",
                at,
            )
            .unwrap_err(),
        PlanRuntimeError::PendingResponse
    );
    // 別 space から配送を問うと対象不一致。
    let source = approval_response::response_execution(&with_response, &prepared, at);
    assert_eq!(
        with_response
            .response_delivery(&prepared, &other_space, &source)
            .unwrap_err(),
        PlanRuntimeError::ResponseTargetMismatch
    );
    assert_eq!(
        with_response
            .response_delivery(
                &PlanApprovalOperationId::generate(),
                &SpaceName::default(),
                &source
            )
            .unwrap_err(),
        PlanRuntimeError::NoPreparedResponse
    );

    // 失効の準備が保留中: 応答の準備は `PendingInvalidation`、未知の失効は解決できない。
    let mut with_invalidation = runtime.clone();
    let invalidation = PlanApprovalOperationId::generate();
    with_invalidation
        .prepare_invalidation(
            PlanInvalidation::new(invalidation.clone(), SpaceName::default(), execution_id),
            at,
        )
        .unwrap();
    assert_eq!(
        with_invalidation
            .prepare_response(
                PlanApprovalOperationId::generate(),
                origin.clone(),
                challenge.session().clone(),
                "1",
                at,
            )
            .unwrap_err(),
        PlanRuntimeError::PendingInvalidation
    );
    assert_eq!(
        with_invalidation
            .resolve_for_publication(
                &PlanApprovalOperationId::generate(),
                &SpaceName::default(),
                &source,
                at,
            )
            .unwrap_err(),
        PlanRuntimeError::UnknownInvalidation
    );
    assert_eq!(
        with_invalidation
            .resolve_for_publication(&invalidation, &other_space, &source, at)
            .unwrap_err(),
        PlanRuntimeError::InvalidationTargetMismatch
    );

    // 回答が監査待ち: 観測は `PendingAnswer`、別 space の中断は対象不一致、
    // 受領のソースが一致している中断は `InvalidState`。
    let mut with_answer = runtime.clone();
    let response = PlanApprovalOperationId::generate();
    with_answer
        .observe_response(response, &offered, challenge.session(), "1", at)
        .unwrap();
    let evidence = challenge.evidence();
    let answered = PlanApprovalEvidence::new(
        evidence.authority().clone(),
        evidence.fingerprint().to_string(),
        evidence.questions_file().to_string(),
        "f".repeat(64),
        evidence.prompt_sha256().to_string(),
    )
    .unwrap();
    let answer_id = PlanApprovalOperationId::generate();
    with_answer
        .record_answer(
            answer_id.clone(),
            PlanAnswerInput::new(
                origin,
                "code-generation".to_string(),
                PlanDecisionEvidence::new(answered, challenge.session().clone()),
                PlanChoice::ApprovePlan,
                Some(evidence.authority().source_floor().to_string()),
            ),
            at,
        )
        .unwrap();
    assert_eq!(
        with_answer
            .observe_response(
                PlanApprovalOperationId::generate(),
                &offered,
                challenge.session(),
                "1",
                at,
            )
            .unwrap_err(),
        PlanRuntimeError::PendingAnswer
    );
    let source = approval_answer::source_execution(&with_answer, &answer_id, at);
    assert_eq!(
        with_answer
            .abort_answer(&answer_id, &other_space, &source, None, at)
            .unwrap_err(),
        PlanRuntimeError::AnswerTargetMismatch
    );
    let certified = with_answer
        .answers()
        .get(&answer_id)
        .unwrap()
        .receipt()
        .unwrap()
        .certified_source()
        .to_string();
    assert_eq!(
        with_answer
            .abort_answer(
                &answer_id,
                &SpaceName::default(),
                &source,
                Some(&certified),
                at
            )
            .unwrap_err(),
        PlanRuntimeError::InvalidState
    );
}

/// 共有側のローカルエンティティは、選択・受領・状態の対応が崩れた再構成を拒む。
#[test]
fn local_entities_refuse_a_receipt_that_disagrees_with_their_choice_or_status() {
    use core_command_domain::orchestration::{
        IntentExecutionId, PlanAnswer, PlanAnswerInput, PlanAnswerState, PlanApprovalEvidence,
        PlanApprovalOperationId, PlanApprovalOrigin, PlanApprovalReceipt, PlanChoice,
        PlanDecisionEvidence, PlanGeneration, PlanGenerationState, PlanGenerationStatus,
    };
    use core_command_domain::workspace::SpaceName;
    let challenge = fixed_challenge();
    let evidence = challenge.evidence();
    let answered = PlanApprovalEvidence::new(
        evidence.authority().clone(),
        evidence.fingerprint().to_string(),
        evidence.questions_file().to_string(),
        "f".repeat(64),
        evidence.prompt_sha256().to_string(),
    )
    .unwrap();
    let decision = PlanDecisionEvidence::new(answered, challenge.session().clone());
    let approved = PlanApprovalReceipt::new(
        decision.clone(),
        challenge.id().to_string(),
        evidence.authority().source_floor().to_string(),
        PlanGenerationStatus::Approved,
    )
    .unwrap();
    // 開始操作は generation 受領しか受け取らない。
    assert_eq!(
        PlanGeneration::new(
            PlanApprovalOperationId::generate(),
            approved.clone(),
            PlanGenerationState::Pending,
        )
        .unwrap_err()
        .to_string(),
        "generation operation requires a generation receipt"
    );
    let input = |choice| {
        PlanAnswerInput::new(
            PlanApprovalOrigin::new(
                SpaceName::default(),
                IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
            ),
            "code-generation".to_string(),
            decision.clone(),
            choice,
            None,
        )
    };
    // 承認の回答に受領が無い / 修正要求に受領が付いている / 受領の提示が別物。
    for (choice, receipt, challenge_id) in [
        (PlanChoice::ApprovePlan, None, challenge.id().to_string()),
        (
            PlanChoice::RequestChanges,
            Some(approved.clone()),
            challenge.id().to_string(),
        ),
        (
            PlanChoice::ApprovePlan,
            Some(approved.clone()),
            "other-challenge".to_string(),
        ),
    ] {
        assert_eq!(
            PlanAnswer::new(
                PlanApprovalOperationId::generate(),
                input(choice),
                PlanApprovalOperationId::generate(),
                challenge_id,
                PlanApprovalOperationId::generate(),
                receipt,
                PlanAnswerState::Pending,
            )
            .unwrap_err()
            .to_string(),
            "plan answer and receipt do not match"
        );
    }
    let answer = PlanAnswer::new(
        PlanApprovalOperationId::generate(),
        input(PlanChoice::ApprovePlan),
        PlanApprovalOperationId::generate(),
        challenge.id().to_string(),
        PlanApprovalOperationId::generate(),
        Some(approved),
        PlanAnswerState::Pending,
    )
    .unwrap();
    assert_eq!(answer.challenge_id(), challenge.id());
    assert_eq!(answer.input().choice(), PlanChoice::ApprovePlan);
}

/// 実行側は、別の実行を観測先にした応答を自分の事実として記録しない。
#[test]
fn a_prepared_response_for_another_execution_is_not_recorded_by_this_one() {
    use core_command_domain::orchestration::{
        CommandError, IntentExecution, IntentExecutionEventId, IntentExecutionId, IntentId,
        PlanApprovalOperationId, PlanApprovalOrigin, PlanApprovalRuntime, StageDisplay,
        StageEntries, StageEntry, Started,
    };
    use core_command_domain::workflow_definition::{PhaseId, PlanAction, StageNumber, StageSlug};
    use core_command_domain::workspace::SpaceName;
    let challenge = fixed_challenge();
    let at = DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let (mut runtime, _) = PlanApprovalRuntime::create(at);
    let offered = PlanApprovalOperationId::generate();
    runtime
        .issue_challenge(offered, challenge.clone(), at)
        .unwrap();
    let id = PlanApprovalOperationId::generate();
    runtime
        .prepare_response(
            id.clone(),
            PlanApprovalOrigin::new(
                SpaceName::default(),
                IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
            ),
            challenge.session().clone(),
            "1",
            at,
        )
        .unwrap();
    let stages = StageEntries::new(vec![StageEntry::new(
        StageSlug::parse("code-generation").unwrap(),
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
    let mut other = IntentExecution::from((
        Started::new(
            IntentExecutionEventId::generate(),
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0001").unwrap(),
            IntentId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0099").unwrap(),
            stages,
        ),
        at,
    ));
    let before = other.clone();
    assert_eq!(
        other
            .record_prepared_response(&runtime, &id, at)
            .unwrap_err(),
        CommandError::PlanResponseTargetMismatch
    );
    assert_eq!(
        other
            .record_prepared_response(&runtime, &PlanApprovalOperationId::generate(), at)
            .unwrap_err(),
        CommandError::PlanResponseUnavailable
    );
    assert_eq!(other, before);
}

/// 回答の記録と開始要求は、保留中の応答準備がある間は拒む。修正要求の回答は受領を持たない。
#[test]
fn recording_an_answer_or_requesting_generation_waits_for_pending_responses() {
    use core_command_domain::orchestration::{
        CodeGenerationApproval, IntentExecutionId, PlanAnswerInput, PlanApprovalDocuments,
        PlanApprovalOperationId, PlanApprovalOrigin, PlanApprovalRuntime, PlanChoice,
        PlanDecisionEvidence, PlanReceipts, PlanTarget,
    };
    use core_command_domain::workspace::SpaceName;
    let challenge = fixed_challenge();
    let at = DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let origin = PlanApprovalOrigin::new(
        SpaceName::default(),
        IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
    );
    let (mut runtime, _) = PlanApprovalRuntime::create(at);
    let offered = PlanApprovalOperationId::generate();
    runtime
        .issue_challenge(offered.clone(), challenge.clone(), at)
        .unwrap();
    let input = |choice| {
        PlanAnswerInput::new(
            origin.clone(),
            "code-generation".to_string(),
            PlanDecisionEvidence::new(challenge.evidence().clone(), challenge.session().clone()),
            choice,
            Some(challenge.evidence().authority().source_floor().to_string()),
        )
    };
    let mut pending = runtime.clone();
    pending
        .prepare_response(
            PlanApprovalOperationId::generate(),
            origin.clone(),
            challenge.session().clone(),
            "1",
            at,
        )
        .unwrap();
    assert_eq!(
        pending
            .record_answer(
                PlanApprovalOperationId::generate(),
                input(PlanChoice::ApprovePlan),
                at
            )
            .unwrap_err()
            .to_string(),
        "pending approval operations must be recovered before recording an answer"
    );
    let approval = CodeGenerationApproval::evaluate(
        Err(core_command_domain::orchestration::PlanTarget::for_unit("").unwrap_err()),
        &PlanTarget::stage_level(),
        &PlanApprovalDocuments::new(
            String::new(),
            String::new(),
            String::new(),
            "q.md".to_string(),
        ),
        core_command_domain::orchestration::TestingPosture::resolve(
            &core_command_domain::orchestration::TestingSections::new(
                String::new(),
                String::new(),
                String::new(),
            ),
            &core_command_domain::orchestration::TestingContext::new(
                "classic".to_string(),
                "minimal".to_string(),
                "greenfield".to_string(),
            ),
        ),
        &PlanReceipts::default(),
        None,
    );
    assert_eq!(
        pending
            .request_generation(PlanApprovalOperationId::generate(), &approval, None, at)
            .unwrap_err()
            .to_string(),
        "pending approval operations must be recovered before generation"
    );
    // 準備が無ければ開始要求は判断の理由をそのまま返す。
    assert_eq!(
        runtime
            .clone()
            .request_generation(PlanApprovalOperationId::generate(), &approval, None, at)
            .unwrap_err()
            .to_string(),
        approval.reason()
    );
    // 修正要求 (`2`) の回答は受領を持たない。
    runtime
        .observe_response(
            PlanApprovalOperationId::generate(),
            &offered,
            challenge.session(),
            "2",
            at,
        )
        .unwrap();
    let id = PlanApprovalOperationId::generate();
    runtime
        .record_answer(id.clone(), input(PlanChoice::RequestChanges), at)
        .unwrap();
    let answer = runtime.answers().get(&id).unwrap();
    assert_eq!(answer.input().choice(), PlanChoice::RequestChanges);
    assert!(answer.receipt().is_none());
    assert!(runtime.receipts().iter().next().is_none());
}

#[path = "../../../../../tests/support/approval_answer.rs"]
mod approval_answer;
