//! 実SQLiteで共有承認の発行回・応答・未完了操作を保存する境界。
#![allow(clippy::unwrap_used)]
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::*;
use core_command_domain::workspace::{SpaceName, StorePath};
use core_command_interface_adapter::orchestration::PlanApprovalRuntimeRepositoryImpl;
use core_command_use_case::orchestration::{PlanApprovalRuntimeRepository, RepositoryError};
fn at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}
fn challenge() -> PlanChallenge {
    // 永続化だけを検査する合成値。本家由来のハッシュ一致はdomainの契約テストが担当する。
    let authority = CodeGenerationAuthority::new(
        &PlanTarget::stage_level(),
        &IntentId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
        format!("sha256:{}", "a".repeat(64)),
        "WORKFLOW_STARTED:2026-09-08T01:00:00Z#1".to_string(),
        "b".repeat(64),
        2,
    )
    .unwrap();
    let evidence = PlanApprovalEvidence::new(authority, format!("sha256:{}", "c".repeat(64)), "aidlc/spaces/default/intents/example/construction/code-generation/code-generation-questions.md".to_string(), "d".repeat(64), "e".repeat(64)).unwrap();
    PlanChallenge::issue(
        evidence,
        PlanSession::new("session".to_string()).unwrap(),
        ["Approve Plan".to_string(), "Request Changes".to_string()],
        false,
    )
}
#[tokio::test]
async fn sqlite_reopening_preserves_observed_issuance_and_pending_invalidation() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    assert_eq!(path.as_path(), temp.path().join(".aidlc-runtime.sqlite"));
    let mut repository = PlanApprovalRuntimeRepositoryImpl::open(&path).unwrap();
    let (runtime, created) = PlanApprovalRuntime::create(at());
    assert!(matches!(
        repository.find_by_id(runtime.id()).await,
        Err(RepositoryError::NotFound { .. })
    ));
    repository.store(&created, &runtime).await.unwrap();
    let mut runtime = repository.find_by_id(runtime.id()).await.unwrap();
    let challenge = challenge();
    let issuance = PlanApprovalOperationId::generate();
    let event = runtime
        .issue_challenge(issuance.clone(), challenge.clone(), at())
        .unwrap();
    repository.store(&event, &runtime).await.unwrap();
    let mut runtime = repository.find_by_id(runtime.id()).await.unwrap();
    let observation = PlanApprovalOperationId::generate();
    let event = runtime
        .observe_response(
            observation.clone(),
            &issuance,
            challenge.session(),
            "1",
            at(),
        )
        .unwrap();
    repository.store(&event, &runtime).await.unwrap();
    let mut runtime = repository.find_by_id(runtime.id()).await.unwrap();
    let operation = PlanApprovalOperationId::generate();
    let event = runtime
        .prepare_invalidation(
            PlanInvalidation::new(
                operation.clone(),
                SpaceName::parse("other-space").unwrap(),
                IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0001").unwrap(),
            ),
            at(),
        )
        .unwrap();
    repository.store(&event, &runtime).await.unwrap();
    drop(repository);
    let repository = PlanApprovalRuntimeRepositoryImpl::open(&path).unwrap();
    let reopened = repository.find_by_id(runtime.id()).await.unwrap();
    assert_eq!(reopened.seq_nr(), 4);
    assert_eq!(
        reopened.invalidations().iter().next().unwrap().id(),
        &operation
    );
    let current = reopened
        .challenges()
        .for_session(challenge.session())
        .unwrap();
    assert_eq!(current.id(), &issuance);
    assert_eq!(current.challenge(), &challenge);
    assert_eq!(current.response().unwrap().id(), &observation);
    assert_eq!(
        current.response().unwrap().choice(),
        PlanChoice::ApprovePlan
    );
    let db = rusqlite::Connection::open(path.as_path()).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM journal", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        4
    );
}

#[tokio::test]
async fn failed_append_and_snapshot_update_leave_no_partial_approval_fact() {
    for (table, action) in [("journal", "INSERT"), ("snapshot", "UPDATE")] {
        let temp = tempfile::tempdir().unwrap();
        let path = StorePath::for_runtime(temp.path());
        let mut repository = PlanApprovalRuntimeRepositoryImpl::open(&path).unwrap();
        let (runtime, created) = PlanApprovalRuntime::create(at());
        repository.store(&created, &runtime).await.unwrap();
        let mut runtime = repository.find_by_id(runtime.id()).await.unwrap();
        let before = runtime.clone();
        let operation = PlanApprovalOperationId::generate();
        let preparation = PlanInvalidation::new(
            operation.clone(),
            SpaceName::parse("default").unwrap(),
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0001").unwrap(),
        );
        let event = runtime
            .prepare_invalidation(preparation.clone(), at())
            .unwrap();
        let db = rusqlite::Connection::open(path.as_path()).unwrap();
        db.execute_batch(&format!("CREATE TRIGGER fail_approval BEFORE {action} ON {table} BEGIN SELECT RAISE(ABORT, 'injected approval failure'); END;")).unwrap();
        assert!(repository.store(&event, &runtime).await.is_err(), "{table}");
        assert_eq!(
            repository.find_by_id(runtime.id()).await.unwrap(),
            before,
            "{table}"
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM journal", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        db.execute_batch("DROP TRIGGER fail_approval").unwrap();
        let mut recovered = repository.find_by_id(runtime.id()).await.unwrap();
        let event = recovered.prepare_invalidation(preparation, at()).unwrap();
        repository.store(&event, &recovered).await.unwrap();
        drop(repository);
        let repository = PlanApprovalRuntimeRepositoryImpl::open(&path).unwrap();
        let mut reopened = repository.find_by_id(runtime.id()).await.unwrap();
        assert_eq!(
            reopened.invalidations().iter().next().unwrap().id(),
            &operation
        );
        assert!(
            reopened
                .issue_challenge(PlanApprovalOperationId::generate(), challenge(), at())
                .is_err()
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM journal", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
    }
}

#[tokio::test]
async fn concurrent_handles_cannot_overwrite_an_unrecovered_invalidation() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let mut repository = PlanApprovalRuntimeRepositoryImpl::open(&path).unwrap();
    let (runtime, created) = PlanApprovalRuntime::create(at());
    repository.store(&created, &runtime).await.unwrap();
    let mut first = repository.find_by_id(runtime.id()).await.unwrap();
    let mut stale = repository.find_by_id(runtime.id()).await.unwrap();
    let operation = PlanApprovalOperationId::generate();
    let prepared = first
        .prepare_invalidation(
            PlanInvalidation::new(
                operation.clone(),
                SpaceName::parse("default").unwrap(),
                IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0001").unwrap(),
            ),
            at(),
        )
        .unwrap();
    repository.store(&prepared, &first).await.unwrap();
    let stale_event = stale
        .issue_challenge(PlanApprovalOperationId::generate(), challenge(), at())
        .unwrap();
    let mut other = PlanApprovalRuntimeRepositoryImpl::open(&path).unwrap();
    assert!(matches!(
        other.store(&stale_event, &stale).await,
        Err(RepositoryError::Conflict {
            expected: 1,
            actual: 2
        })
    ));
    let mut current = other.find_by_id(runtime.id()).await.unwrap();
    assert_eq!(
        current.invalidations().iter().next().unwrap().id(),
        &operation
    );
    assert!(
        current
            .issue_challenge(PlanApprovalOperationId::generate(), challenge(), at())
            .is_err()
    );
}

#[tokio::test]
async fn the_latest_snapshot_is_replayed_only_with_its_later_events() {
    use core_command_interface_adapter::orchestration::{
        PlanApprovalEventDto, PlanApprovalRuntimeDto, PlanApprovalRuntimeKeyDto,
    };
    use event_store_adapter_rs::{
        EventStoreForSqlite, event_envelope::EventEnvelope, types::EventStore,
    };
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let mut repository = PlanApprovalRuntimeRepositoryImpl::open(&path).unwrap();
    let (runtime, created) = PlanApprovalRuntime::create(at());
    repository.store(&created, &runtime).await.unwrap();
    let mut runtime = repository.find_by_id(runtime.id()).await.unwrap();
    let challenge = challenge();
    let issuance = PlanApprovalOperationId::generate();
    let issued = runtime
        .issue_challenge(issuance.clone(), challenge.clone(), at())
        .unwrap();
    repository.store(&issued, &runtime).await.unwrap();
    let mut runtime = repository.find_by_id(runtime.id()).await.unwrap();
    let mut raw = EventStoreForSqlite::<
        PlanApprovalRuntimeKeyDto,
        PlanApprovalRuntimeDto,
        PlanApprovalEventDto,
    >::new(path.as_path())
    .unwrap();
    let observed = runtime
        .observe_response(
            PlanApprovalOperationId::generate(),
            &issuance,
            challenge.session(),
            "1",
            at(),
        )
        .unwrap();
    raw.persist_event(
        EventEnvelope::new(
            PlanApprovalRuntimeKeyDto::of(runtime.id()),
            3,
            at(),
            PlanApprovalEventDto::of(&observed),
        )
        .with_manifest("plan-approval-event/1"),
        2,
    )
    .await
    .unwrap();
    let operation = PlanApprovalOperationId::generate();
    let prepared = runtime
        .prepare_invalidation(
            PlanInvalidation::new(
                operation.clone(),
                SpaceName::parse("second").unwrap(),
                IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0001").unwrap(),
            ),
            at(),
        )
        .unwrap();
    raw.persist_event(
        EventEnvelope::new(
            PlanApprovalRuntimeKeyDto::of(runtime.id()),
            4,
            at(),
            PlanApprovalEventDto::of(&prepared),
        )
        .with_manifest("plan-approval-event/1"),
        3,
    )
    .await
    .unwrap();
    let (space, source) = approval_publication::source_execution(&runtime, &operation, true, at());
    let resolved = runtime
        .resolve_for_publication(&operation, &space, &source, at())
        .unwrap();
    raw.persist_event(
        EventEnvelope::new(
            PlanApprovalRuntimeKeyDto::of(runtime.id()),
            5,
            at(),
            PlanApprovalEventDto::of(&resolved),
        )
        .with_manifest("plan-approval-event/1"),
        4,
    )
    .await
    .unwrap();
    drop(raw);
    let repository = PlanApprovalRuntimeRepositoryImpl::open(&path).unwrap();
    assert_eq!(
        repository.find_by_id(runtime.id()).await.unwrap(),
        runtime.with_version(5)
    );
}

#[path = "../../../../../tests/support/approval_publication.rs"]
mod approval_publication;

#[tokio::test]
async fn a_prepared_response_retains_its_original_offer_after_reopening() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let mut repository = PlanApprovalRuntimeRepositoryImpl::open(&path).unwrap();
    let (runtime, created) = PlanApprovalRuntime::create(at());
    repository.store(&created, &runtime).await.unwrap();
    let mut runtime = repository.find_by_id(runtime.id()).await.unwrap();
    let challenge = challenge();
    let offered = PlanApprovalOperationId::generate();
    let event = runtime
        .issue_challenge(offered.clone(), challenge.clone(), at())
        .unwrap();
    repository.store(&event, &runtime).await.unwrap();
    let mut runtime = repository.find_by_id(runtime.id()).await.unwrap();
    let id = PlanApprovalOperationId::generate();
    let origin = PlanApprovalOrigin::new(
        SpaceName::parse("other").unwrap(),
        IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
    );
    let event = runtime
        .prepare_response(
            id.clone(),
            origin.clone(),
            challenge.session().clone(),
            "1",
            at(),
        )
        .unwrap();
    repository.store(&event, &runtime).await.unwrap();
    drop(repository);
    let reopened = PlanApprovalRuntimeRepositoryImpl::open(&path)
        .unwrap()
        .find_by_id(runtime.id())
        .await
        .unwrap();
    let pending = reopened
        .pending_responses()
        .get(&id)
        .expect("元の発行回に結び付けた観測を保持する");
    assert_eq!(pending.occurrence_id(), &offered);
    assert_eq!(pending.origin(), &origin);
    assert_eq!(pending.session(), challenge.session());
    assert_eq!(pending.response(), "1");
    assert_eq!(pending.choice(), Some(PlanChoice::ApprovePlan));
}

#[tokio::test]
async fn sqlite_reopening_preserves_pending_receipt_and_completed_answer() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let mut repository = PlanApprovalRuntimeRepositoryImpl::open(&path).unwrap();
    let (runtime, event) = PlanApprovalRuntime::create(at());
    repository.store(&event, &runtime).await.unwrap();
    let mut runtime = repository.find_by_id(runtime.id()).await.unwrap();
    let offered = challenge();
    let occurrence = PlanApprovalOperationId::generate();
    let event = runtime
        .issue_challenge(occurrence.clone(), offered.clone(), at())
        .unwrap();
    repository.store(&event, &runtime).await.unwrap();
    let mut runtime = repository.find_by_id(runtime.id()).await.unwrap();
    let event = runtime
        .observe_response(
            PlanApprovalOperationId::generate(),
            &occurrence,
            offered.session(),
            "1",
            at(),
        )
        .unwrap();
    repository.store(&event, &runtime).await.unwrap();
    let mut runtime = repository.find_by_id(runtime.id()).await.unwrap();
    let id = PlanApprovalOperationId::generate();
    let input = PlanAnswerInput::new(
        PlanApprovalOrigin::new(
            SpaceName::default(),
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0001").unwrap(),
        ),
        "code-generation".to_string(),
        PlanDecisionEvidence::new(offered.evidence().clone(), offered.session().clone()),
        PlanChoice::ApprovePlan,
        Some(offered.evidence().authority().source_floor().to_string()),
    );
    let event = runtime.record_answer(id.clone(), input, at()).unwrap();
    repository.store(&event, &runtime).await.unwrap();
    let expected = runtime.answers().get(&id).unwrap().clone();
    drop(repository);
    let mut repository = PlanApprovalRuntimeRepositoryImpl::open(&path).unwrap();
    let mut restored = repository.find_by_id(runtime.id()).await.unwrap();
    assert_eq!(restored.answers().get(&id), Some(&expected));
    assert_eq!(restored.receipts(), runtime.receipts());
    let mut source = approval_answer::source_execution(&restored, &id, at());
    let source_hash = expected.receipt().unwrap().certified_source();
    source
        .record_plan_answer(
            &restored,
            &id,
            &SpaceName::default(),
            Some(source_hash),
            at(),
        )
        .unwrap();
    let event = restored
        .complete_answer(&id, &SpaceName::default(), &source, at())
        .unwrap();
    repository.store(&event, &restored).await.unwrap();
    drop(repository);
    let repository = PlanApprovalRuntimeRepositoryImpl::open(&path).unwrap();
    let completed = repository.find_by_id(restored.id()).await.unwrap();
    assert_eq!(
        completed.answers().get(&id).unwrap().state(),
        &PlanAnswerState::Recorded
    );
    assert!(
        completed
            .challenges()
            .for_session(offered.session())
            .is_none()
    );
    assert_eq!(completed.receipts(), restored.receipts());
}
#[path = "../../../../../tests/support/approval_answer.rs"]
mod approval_answer;

#[tokio::test]
async fn generation_publication_and_certification_survive_reopening() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let (expected, events, id, at) = generation_fixture::pending();
    let mut repository = PlanApprovalRuntimeRepositoryImpl::open(&path).unwrap();
    for event in events {
        let runtime = if let PlanApprovalEvent::Created(created) = &event {
            PlanApprovalRuntime::from((created.clone(), at))
        } else {
            let mut runtime = repository
                .find_by_id(&PlanApprovalRuntimeId::Workspace)
                .await
                .unwrap();
            runtime.apply_event(&event, runtime.seq_nr() + 1, at);
            runtime
        };
        repository.store(&event, &runtime).await.unwrap();
    }
    drop(repository);
    let mut repository = PlanApprovalRuntimeRepositoryImpl::open(&path).unwrap();
    let mut restored = repository
        .find_by_id(&PlanApprovalRuntimeId::Workspace)
        .await
        .unwrap();
    assert_eq!(
        restored.generations().get(&id),
        expected.generations().get(&id)
    );
    assert_eq!(
        restored.generations().get(&id).unwrap().state(),
        PlanGenerationState::Pending
    );
    let source = restored
        .generations()
        .get(&id)
        .unwrap()
        .receipt()
        .certified_source()
        .to_string();
    let event = restored.certify_generation(&id, Some(&source), at).unwrap();
    repository.store(&event, &restored).await.unwrap();
    drop(repository);
    let repository = PlanApprovalRuntimeRepositoryImpl::open(&path).unwrap();
    let active = repository
        .find_by_id(&PlanApprovalRuntimeId::Workspace)
        .await
        .unwrap();
    assert_eq!(
        active.generations().get(&id).unwrap().state(),
        PlanGenerationState::Active
    );
    assert!(active.generations().pending().next().is_none());
    assert_eq!(active.receipts(), restored.receipts());
}
#[path = "../../../../../tests/support/generation_fixture.rs"]
mod generation_fixture;
