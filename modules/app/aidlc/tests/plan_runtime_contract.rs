//! Command保存→共有RMU→操作IDによる読取の縦結合。
#![allow(clippy::unwrap_used)]
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    IntentExecutionId, PlanApprovalOperationId, PlanApprovalRuntime, PlanInvalidation,
};
use core_command_domain::workspace::{
    HookHealth, HookHealthTarget, HookName, SpaceName, StorePath,
};
use core_command_interface_adapter::orchestration::{
    HookHealthRepositoryImpl, PlanApprovalRuntimeRepositoryImpl,
};
use core_command_use_case::orchestration::{HookHealthRepository, PlanApprovalRuntimeRepository};
use core_read_model_updater::orchestration::{
    PlanApprovalJournalReaderImpl, PlanApprovalReadModelUpdater, ReadModelUpdater,
};
#[tokio::test]
async fn persisted_operations_are_projected_by_id_in_the_same_runtime_store() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let at = DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let mut repository = PlanApprovalRuntimeRepositoryImpl::open(&path).unwrap();
    let (runtime, created) = PlanApprovalRuntime::create(at);
    repository.store(&created, &runtime).await.unwrap();
    let id = PlanApprovalOperationId::generate();
    let mut command =
        core_command_use_case::orchestration::PreparePlanInvalidationUseCase::new(repository);
    let result: Result<(), _> = command
        .execute(
            PlanInvalidation::new(
                id.clone(),
                SpaceName::parse("other").unwrap(),
                IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
            ),
            at,
        )
        .await;
    result.unwrap();
    let reader = PlanApprovalJournalReaderImpl::open(&path).unwrap();
    let mut updater = PlanApprovalReadModelUpdater::new(reader);
    updater.update_read_models().await.unwrap();
    let daos = core_query_interface_adapter::ReadModelDaos::open(path.as_path()).unwrap();
    let query = core_query_use_case::orchestration::PlanApprovalOperationUseCase::new(
        daos.plan_approval_operation(),
    );
    let view = query
        .execute(id.as_str())
        .unwrap()
        .expect("保存した操作IDの投影");
    assert_eq!(view.id(), id.as_str());
    assert_eq!(view.status(), "prepared");
    assert_eq!(view.space(), Some("other"));
    assert_eq!(
        view.execution_id(),
        Some("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000")
    );
    assert_eq!(view.as_of(), 2);
    assert_eq!(query.pending().unwrap(), vec![view]);
    assert!(
        query
            .execute(PlanApprovalOperationId::generate().as_str())
            .unwrap()
            .is_none()
    );
    let db = rusqlite::Connection::open(path.as_path()).unwrap();
    let row = db
        .query_row(
            "SELECT status, space, execution_id FROM read_plan_operation WHERE operation_id=?1",
            [id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(
        row,
        (
            "prepared".to_string(),
            Some("other".to_string()),
            Some("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000".to_string())
        )
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM journal", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(db.query_row("SELECT last_seq FROM amadeus_plan_projection_checkpoint WHERE projection='plan-approval'", [], |row| row.get::<_,i64>(0)).unwrap(), 2);
}

#[tokio::test]
async fn plan_reader_ignores_an_interleaved_hook_health_stream() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let at: DateTime<Utc> = "2026-09-08T01:00:00Z".parse().unwrap();
    let target = HookHealthTarget::new(SpaceName::parse("default").unwrap(), None);
    let hook = HookName::parse("write-audit-log").unwrap();
    let (health, event) = HookHealth::start(target, hook, at).unwrap();
    let mut health_repo = HookHealthRepositoryImpl::open(&path).unwrap();
    health_repo.store(&event, &health).await.unwrap();
    let (runtime, created) = PlanApprovalRuntime::create(at);
    let mut approval_repo = PlanApprovalRuntimeRepositoryImpl::open(&path).unwrap();
    approval_repo.store(&created, &runtime).await.unwrap();
    let reader = PlanApprovalJournalReaderImpl::open(&path).unwrap();
    let mut updater = PlanApprovalReadModelUpdater::new(reader);
    updater.update_read_models().await.unwrap();
    let db = rusqlite::Connection::open(path.as_path()).unwrap();
    assert_eq!(
        db.query_row::<i64, _, _>("SELECT count(*) FROM journal", [], |row| row.get(0))
            .unwrap(),
        2
    );
}

#[tokio::test]
async fn query_keeps_the_previous_projection_until_recovery_commits_all_rows_and_checkpoint() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let at = DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let mut repository = PlanApprovalRuntimeRepositoryImpl::open(&path).unwrap();
    let (runtime, created) = PlanApprovalRuntime::create(at);
    repository.store(&created, &runtime).await.unwrap();
    let mut runtime = repository.find_by_id(runtime.id()).await.unwrap();
    let id = PlanApprovalOperationId::generate();
    let event = runtime
        .prepare_invalidation(
            PlanInvalidation::new(
                id.clone(),
                SpaceName::parse("other").unwrap(),
                IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
            ),
            at,
        )
        .unwrap();
    repository.store(&event, &runtime).await.unwrap();
    let mut updater =
        PlanApprovalReadModelUpdater::new(PlanApprovalJournalReaderImpl::open(&path).unwrap());
    updater.update_read_models().await.unwrap();
    let daos = core_query_interface_adapter::ReadModelDaos::open(path.as_path()).unwrap();
    let query = core_query_use_case::orchestration::PlanApprovalOperationUseCase::new(
        daos.plan_approval_operation(),
    );
    let prior = query.execute(id.as_str()).unwrap().unwrap();
    let mut runtime = repository.find_by_id(runtime.id()).await.unwrap();
    let (space, source) = approval_publication::source_execution(&runtime, &id, true, at);
    let event = runtime
        .resolve_for_publication(&id, &space, &source, at)
        .unwrap();
    repository.store(&event, &runtime).await.unwrap();
    assert_eq!(
        query.execute(id.as_str()).unwrap(),
        Some(prior.clone()),
        "Queryはイベントから状態を再計算しない"
    );
    let db = rusqlite::Connection::open(path.as_path()).unwrap();
    db.execute_batch("CREATE TRIGGER fail_plan_projection BEFORE INSERT ON read_plan_operation BEGIN SELECT RAISE(ABORT, 'injected projection failure'); END;").unwrap();
    assert!(updater.update_read_models().await.is_err());
    assert_eq!(query.execute(id.as_str()).unwrap(), Some(prior));
    assert_eq!(db.query_row("SELECT last_seq FROM amadeus_plan_projection_checkpoint WHERE projection='plan-approval'", [], |row| row.get::<_,i64>(0)).unwrap(), 2);
    db.execute_batch("DROP TRIGGER fail_plan_projection")
        .unwrap();
    drop(updater);
    let mut recovered =
        PlanApprovalReadModelUpdater::new(PlanApprovalJournalReaderImpl::open(&path).unwrap());
    recovered.update_read_models().await.unwrap();
    let current = query.execute(id.as_str()).unwrap().unwrap();
    assert_eq!(current.id(), id.as_str());
    assert_eq!(current.status(), "applied");
    assert_eq!(current.as_of(), 3);
    assert_eq!(current.space(), None);
    assert!(query.pending().unwrap().is_empty());
    recovered.update_read_models().await.unwrap();
    assert_eq!(query.execute(id.as_str()).unwrap(), Some(current));
    assert_eq!(
        db.query_row("SELECT count(*) FROM journal", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        3
    );
    assert_eq!(db.query_row("SELECT last_seq FROM amadeus_plan_projection_checkpoint WHERE projection='plan-approval'", [], |row| row.get::<_,i64>(0)).unwrap(), 3);
}

#[path = "../../../../tests/support/approval_publication.rs"]
mod approval_publication;

#[tokio::test]
async fn plan_answer_result_is_read_by_its_id_after_projection_and_recovery() {
    use core_command_domain::orchestration::{PlanApprovalEvent, PlanApprovalRuntimeId};
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let at = DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let (_, events, id) = pending_plan_answer::pending_answer(at);
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
    let mut updater =
        PlanApprovalReadModelUpdater::new(PlanApprovalJournalReaderImpl::open(&path).unwrap());
    updater.update_read_models().await.unwrap();
    let daos = core_query_interface_adapter::ReadModelDaos::open(path.as_path()).unwrap();
    let query = core_query_use_case::orchestration::PlanAnswerUseCase::new(daos.plan_answer());
    let pending = query
        .execute(id.as_str())
        .unwrap()
        .expect("指定IDの回答候補");
    assert_eq!(pending.id(), id.as_str());
    assert_eq!(pending.status(), "pending");
    assert_eq!(pending.emitted(), None);
    assert_eq!(pending.stage(), "code-generation");
    assert_eq!(pending.as_of(), 4);
    assert!(
        query
            .execute(PlanApprovalOperationId::generate().as_str())
            .unwrap()
            .is_none()
    );
    let mut runtime = repository
        .find_by_id(&PlanApprovalRuntimeId::Workspace)
        .await
        .unwrap();
    let mut source = approval_answer::source_execution(&runtime, &id, at);
    source
        .record_plan_answer(
            &runtime,
            &id,
            &SpaceName::default(),
            Some(&"b".repeat(64)),
            at,
        )
        .unwrap();
    let event = runtime
        .complete_answer(&id, &SpaceName::default(), &source, at)
        .unwrap();
    repository.store(&event, &runtime).await.unwrap();
    assert_eq!(query.execute(id.as_str()).unwrap(), Some(pending));
    drop(updater);
    let mut recovered =
        PlanApprovalReadModelUpdater::new(PlanApprovalJournalReaderImpl::open(&path).unwrap());
    recovered.update_read_models().await.unwrap();
    let completed = query.execute(id.as_str()).unwrap().unwrap();
    assert_eq!(completed.status(), "recorded");
    assert_eq!(completed.emitted(), Some("PLAN_APPROVAL_RECORDED"));
    assert_eq!(completed.error(), None);
    assert_eq!(completed.as_of(), 5);
    recovered.update_read_models().await.unwrap();
    assert_eq!(query.execute(id.as_str()).unwrap(), Some(completed));
}
#[path = "../../../../tests/support/approval_answer.rs"]
mod approval_answer;

#[path = "../../../../tests/support/pending_plan_answer.rs"]
mod pending_plan_answer;

#[tokio::test]
async fn generation_query_observes_publication_then_certification_by_the_same_id() {
    use core_command_domain::orchestration::{PlanApprovalEvent, PlanApprovalRuntimeId};
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let (_, events, id, at) = generation_fixture::pending();
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
    let mut updater =
        PlanApprovalReadModelUpdater::new(PlanApprovalJournalReaderImpl::open(&path).unwrap());
    updater.update_read_models().await.unwrap();
    let daos = core_query_interface_adapter::ReadModelDaos::open(path.as_path()).unwrap();
    let query =
        core_query_use_case::orchestration::PlanGenerationUseCase::new(daos.plan_generation());
    let pending = query.execute(id.as_str()).unwrap().unwrap();
    assert_eq!(pending.id(), id.as_str());
    assert_eq!(pending.status(), "pending");
    assert_eq!(pending.unit(), None);
    assert_eq!(pending.error(), None);
    let operation = core_query_use_case::orchestration::PlanApprovalOperationUseCase::new(
        daos.plan_approval_operation(),
    )
    .execute(id.as_str())
    .unwrap()
    .unwrap();
    assert_eq!(operation.status(), "prepared");
    assert_eq!(operation.kind(), "generation");
    let mut runtime = repository
        .find_by_id(&PlanApprovalRuntimeId::Workspace)
        .await
        .unwrap();
    let source = runtime
        .generations()
        .get(&id)
        .unwrap()
        .receipt()
        .certified_source()
        .to_string();
    let event = runtime.certify_generation(&id, Some(&source), at).unwrap();
    repository.store(&event, &runtime).await.unwrap();
    assert_eq!(query.execute(id.as_str()).unwrap(), Some(pending));
    drop(updater);
    let mut recovered =
        PlanApprovalReadModelUpdater::new(PlanApprovalJournalReaderImpl::open(&path).unwrap());
    recovered.update_read_models().await.unwrap();
    let active = query.execute(id.as_str()).unwrap().unwrap();
    assert_eq!(active.status(), "generation");
    assert_eq!(active.error(), None);
    assert!(
        query
            .execute(PlanApprovalOperationId::generate().as_str())
            .unwrap()
            .is_none()
    );
}
#[path = "../../../../tests/support/generation_fixture.rs"]
mod generation_fixture;
