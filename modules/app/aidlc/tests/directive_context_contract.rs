//! 文脈失効の実SQLite→RMU→操作ID読取の縦結合。
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use core_command_domain::orchestration::*;
use core_command_domain::workflow_definition::StageSlug;
use core_command_domain::workspace::{SpaceName, StorePath};
use core_command_interface_adapter::orchestration::{
    IntentExecutionRepositoryImpl, PlanApprovalRuntimeRepositoryImpl,
};
use core_command_use_case::orchestration::{
    IntentExecutionRepository, InvalidateDirectiveContextUseCase, PlanApprovalRuntimeRepository,
};
use core_read_model_updater::orchestration::{
    GlobalSeqNr, JournalReader, JournalReaderImpl, PlanApprovalJournalReaderImpl,
    PlanApprovalReadModelUpdater,
};

#[tokio::test]
async fn context_invalidation_projects_error_and_preserves_the_original_source_authority_revision()
{
    let temp = tempfile::tempdir().unwrap();
    let space = SpaceName::default();
    let source_path = StorePath::for_space(temp.path(), &space);
    std::fs::create_dir_all(source_path.as_path().parent().unwrap()).unwrap();
    let runtime_path = StorePath::for_runtime(temp.path());
    let mut source = IntentExecutionRepositoryImpl::open(&source_path).unwrap();
    let (execution, genesis) = support::genesis();
    source.store(&genesis, &execution).await.unwrap();
    let mut execution = source.find_by_id(execution.id()).await.unwrap();
    let publication = DirectivePublication::new(
        "a".repeat(64),
        "b".repeat(64),
        PublishedDirective::RunStage {
            stage: StageSlug::parse("code-generation").unwrap(),
            unit: None,
        },
    )
    .with_source_floor(Some("c".repeat(64)));
    let event = execution
        .issue_directive(&publication, support::at())
        .unwrap();
    source.store(&event, &execution).await.unwrap();
    let original_revision = execution.active_directive().unwrap().revision();
    let mut runtime_repository = PlanApprovalRuntimeRepositoryImpl::open(&runtime_path).unwrap();
    let (runtime, event) = PlanApprovalRuntime::create(support::at());
    runtime_repository.store(&event, &runtime).await.unwrap();
    let mut runtime = runtime_repository.find_by_id(runtime.id()).await.unwrap();
    let event = runtime
        .issue_challenge(
            PlanApprovalOperationId::generate(),
            challenge(),
            support::at(),
        )
        .unwrap();
    runtime_repository.store(&event, &runtime).await.unwrap();

    let operation = PlanApprovalOperationId::generate();
    let request = DirectiveContextInvalidation::new(
        execution.intent_id().clone(),
        "a".repeat(64),
        "b".repeat(64),
        "sessionless:aaaaaaaaaaaaaaaa".into(),
    );
    let result: Result<(), _> = InvalidateDirectiveContextUseCase::new(runtime_repository, source)
        .execute(execution.id(), &space, &operation, &request, support::at())
        .await;
    result.unwrap();
    let recovered_runtime = PlanApprovalRuntimeRepositoryImpl::open(&runtime_path)
        .unwrap()
        .find_by_id(runtime.id())
        .await
        .unwrap();
    assert!(recovered_runtime.challenges().iter().next().is_none());
    let reader = JournalReaderImpl::open(&source_path).unwrap();
    let batch = reader.events_after(GlobalSeqNr::ZERO).await.unwrap();
    let entries: Vec<_> = batch
        .executions()
        .iter()
        .filter(|entry| {
            matches!(
                entry.event(),
                IntentExecutionEvent::DirectiveIssued(_)
                    | IntentExecutionEvent::DirectiveContextInvalidated(_)
            )
        })
        .cloned()
        .collect();
    assert_eq!(entries.len(), 2);
    let mut view = core_read_model_updater::workspace::ReadModel::new("");
    core_read_model_updater::workspace::project(
        &entries,
        &core_read_model_updater::workspace::ResolvedPlan::of(&support::intent()),
        &mut view,
    )
    .unwrap();
    let marker: serde_json::Value = serde_json::from_str(view.active_directive().unwrap()).unwrap();
    assert_eq!(marker.get("kind").unwrap(), "error");
    assert_eq!(marker.get("context_epoch").unwrap(), 1);
    assert_eq!(marker.get("revision").unwrap(), original_revision + 1);
    assert_eq!(
        marker.get("owner_session").unwrap(),
        "sessionless:aaaaaaaaaaaaaaaa"
    );
    assert_eq!(marker.get("delivery").unwrap(), "superseded");
    assert_eq!(marker.get("needs_rehydrate").unwrap(), true);
    assert_eq!(
        marker.get("code_generation_authority_revision").unwrap(),
        original_revision
    );
    assert!(view.appended_audit().is_empty());
    let mut updater = PlanApprovalReadModelUpdater::new(
        PlanApprovalJournalReaderImpl::open(&runtime_path).unwrap(),
    );
    updater.catch_up().unwrap();
    let daos = core_query_interface_adapter::ReadModelDaos::open(runtime_path.as_path()).unwrap();
    let query = core_query_use_case::orchestration::PlanApprovalOperationUseCase::new(
        daos.plan_approval_operation(),
    );
    let operation = query.execute(operation.as_str()).unwrap().unwrap();
    assert_eq!(operation.status(), "applied");
    assert!(query.pending().unwrap().is_empty());
}

#[tokio::test]
async fn foreign_empty_missing_or_stale_contexts_leave_both_stores_unchanged() {
    for mismatch in [
        "session",
        "empty-session",
        "project",
        "intent",
        "state",
        "missing",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let space = SpaceName::default();
        let source_path = StorePath::for_space(temp.path(), &space);
        std::fs::create_dir_all(source_path.as_path().parent().unwrap()).unwrap();
        let runtime_path = StorePath::for_runtime(temp.path());
        let mut source = IntentExecutionRepositoryImpl::open(&source_path).unwrap();
        let (execution, genesis) = support::genesis();
        source.store(&genesis, &execution).await.unwrap();
        let mut execution = source.find_by_id(execution.id()).await.unwrap();
        if mismatch != "missing" {
            let publication = DirectivePublication::new(
                "a".repeat(64),
                "b".repeat(64),
                PublishedDirective::RunStage {
                    stage: StageSlug::parse("code-generation").unwrap(),
                    unit: None,
                },
            );
            let event = execution
                .issue_directive(&publication, support::at())
                .unwrap();
            source.store(&event, &execution).await.unwrap();
        }
        let before = source.find_by_id(execution.id()).await.unwrap();
        let mut repository = PlanApprovalRuntimeRepositoryImpl::open(&runtime_path).unwrap();
        let (runtime, event) = PlanApprovalRuntime::create(support::at());
        repository.store(&event, &runtime).await.unwrap();
        let mut runtime = repository.find_by_id(runtime.id()).await.unwrap();
        let offered = challenge();
        let event = runtime
            .issue_challenge(PlanApprovalOperationId::generate(), offered, support::at())
            .unwrap();
        repository.store(&event, &runtime).await.unwrap();
        let before_runtime = repository.find_by_id(runtime.id()).await.unwrap();
        let request = DirectiveContextInvalidation::new(
            if mismatch == "intent" {
                IntentId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0099").unwrap()
            } else {
                execution.intent_id().clone()
            },
            if mismatch == "project" { "c" } else { "a" }.repeat(64),
            if mismatch == "state" { "c" } else { "b" }.repeat(64),
            match mismatch {
                "session" => "foreign",
                "empty-session" => "",
                _ => "sessionless:aaaaaaaaaaaaaaaa",
            }
            .into(),
        );
        InvalidateDirectiveContextUseCase::new(repository, source)
            .execute(
                execution.id(),
                &space,
                &PlanApprovalOperationId::generate(),
                &request,
                support::at(),
            )
            .await
            .unwrap();
        let recovered = IntentExecutionRepositoryImpl::open(&source_path)
            .unwrap()
            .find_by_id(execution.id())
            .await
            .unwrap();
        let recovered_runtime = PlanApprovalRuntimeRepositoryImpl::open(&runtime_path)
            .unwrap()
            .find_by_id(runtime.id())
            .await
            .unwrap();
        assert_eq!(recovered, before, "{mismatch}: source");
        assert_eq!(recovered_runtime, before_runtime, "{mismatch}: approval");
    }
}

fn challenge() -> PlanChallenge {
    let authority = CodeGenerationAuthority::new(
        &PlanTarget::stage_level(),
        &support::intent_id(),
        format!("sha256:{}", "d".repeat(64)),
        "unstarted#0".into(),
        "c".repeat(64),
        1,
    )
    .unwrap();
    let evidence = PlanApprovalEvidence::new(
        authority,
        format!("sha256:{}", "e".repeat(64)),
        "questions.md".into(),
        "f".repeat(64),
        "a".repeat(64),
    )
    .unwrap();
    PlanChallenge::issue(
        evidence,
        PlanSession::new("human-session".into()).unwrap(),
        ["Approve Plan".into(), "Request Changes".into()],
        false,
    )
}

#[tokio::test]
async fn recovery_resets_shared_challenges_only_when_the_source_committed_invalidation() {
    use core_command_use_case::orchestration::RecoverPlanInvalidationUseCase;
    for committed in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let space = SpaceName::default();
        let source_path = StorePath::for_space(temp.path(), &space);
        std::fs::create_dir_all(source_path.as_path().parent().unwrap()).unwrap();
        let runtime_path = StorePath::for_runtime(temp.path());
        let mut source = IntentExecutionRepositoryImpl::open(&source_path).unwrap();
        let (execution, genesis) = support::genesis();
        source.store(&genesis, &execution).await.unwrap();
        let mut execution = source.find_by_id(execution.id()).await.unwrap();
        let publication = DirectivePublication::new(
            "a".repeat(64),
            "b".repeat(64),
            PublishedDirective::RunStage {
                stage: StageSlug::parse("code-generation").unwrap(),
                unit: None,
            },
        );
        let event = execution
            .issue_directive(&publication, support::at())
            .unwrap();
        source.store(&event, &execution).await.unwrap();
        let mut execution = source.find_by_id(execution.id()).await.unwrap();
        let mut repository = PlanApprovalRuntimeRepositoryImpl::open(&runtime_path).unwrap();
        let (runtime, event) = PlanApprovalRuntime::create(support::at());
        repository.store(&event, &runtime).await.unwrap();
        let mut runtime = repository.find_by_id(runtime.id()).await.unwrap();
        let event = runtime
            .issue_challenge(
                PlanApprovalOperationId::generate(),
                challenge(),
                support::at(),
            )
            .unwrap();
        repository.store(&event, &runtime).await.unwrap();
        let mut runtime = repository.find_by_id(runtime.id()).await.unwrap();
        let operation = PlanApprovalOperationId::generate();
        let event = runtime
            .prepare_invalidation(
                PlanInvalidation::new(operation.clone(), space.clone(), execution.id().clone()),
                support::at(),
            )
            .unwrap();
        repository.store(&event, &runtime).await.unwrap();
        if committed {
            let request = DirectiveContextInvalidation::new(
                execution.intent_id().clone(),
                "a".repeat(64),
                "b".repeat(64),
                "sessionless:aaaaaaaaaaaaaaaa".into(),
            );
            let event = execution
                .invalidate_directive_context(&operation, &request, support::at())
                .unwrap();
            source.store(&event, &execution).await.unwrap();
        }
        drop(repository);
        drop(source);
        RecoverPlanInvalidationUseCase::new(
            PlanApprovalRuntimeRepositoryImpl::open(&runtime_path).unwrap(),
            IntentExecutionRepositoryImpl::open(&source_path).unwrap(),
        )
        .execute(&operation, &space, execution.id(), support::at())
        .await
        .unwrap();
        let recovered = PlanApprovalRuntimeRepositoryImpl::open(&runtime_path)
            .unwrap()
            .find_by_id(runtime.id())
            .await
            .unwrap();
        assert_eq!(recovered.challenges().iter().next().is_none(), committed);
        assert!(recovered.invalidations().iter().next().is_none());
        assert!(
            recovered
                .applied_operations()
                .iter()
                .any(|id| id == &operation)
        );
    }
}
