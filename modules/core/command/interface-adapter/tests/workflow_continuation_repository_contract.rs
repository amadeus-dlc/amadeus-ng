//! 同じ停止判断・公開確定・競合契約を本家SQLite/memoryへ課す。
#![allow(clippy::unwrap_used)]
use core_command_domain::orchestration::{
    ContinuationAttemptId, ContinuationRequest, ContinuationSignature, IntentExecutionId,
    WorkflowContinuation, WorkflowContinuationId,
};
use core_command_domain::workspace::StorePath;
use core_command_interface_adapter::orchestration::WorkflowContinuationRepositoryImpl;
use core_command_use_case::orchestration::{RepositoryError, WorkflowContinuationRepository};

async fn contract(mut repository: impl WorkflowContinuationRepository) {
    let id = WorkflowContinuationId::for_execution(
        &IntentExecutionId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0001").unwrap(),
    );
    assert!(matches!(
        repository.find_by_id(&id).await,
        Err(RepositoryError::NotFound { .. })
    ));
    let request = ContinuationRequest::new(
        ContinuationAttemptId::generate(),
        Some(
            ContinuationSignature::parse(&format!(
                "reverse-engineering::{}::{}",
                "a".repeat(64),
                "b".repeat(64)
            ))
            .unwrap(),
        ),
        false,
        2,
    )
    .unwrap();
    let attempt = request.id().clone();
    let (original, event) =
        WorkflowContinuation::start(id.clone(), request, chrono::Utc::now()).unwrap();
    repository.store(&event, &original).await.unwrap();
    let mut restored = repository.find_by_id(&id).await.unwrap();
    assert_eq!(restored.last_request(), original.last_request());
    assert_eq!(restored.counter(), original.counter());
    let mut stale = restored.clone();
    let failed = restored
        .record_publication(
            &core_command_domain::orchestration::ContinuationPublicationObservation::new(
                (attempt).clone(),
                false,
            ),
            chrono::Utc::now(),
        )
        .unwrap();
    repository.store(&failed, &restored).await.unwrap();
    let conflicting = stale
        .record_publication(
            &core_command_domain::orchestration::ContinuationPublicationObservation::new(
                (attempt).clone(),
                true,
            ),
            chrono::Utc::now(),
        )
        .unwrap();
    assert!(matches!(
        repository.store(&conflicting, &stale).await,
        Err(RepositoryError::Conflict {
            expected: 1,
            actual: 2
        })
    ));
    let restored = repository.find_by_id(&id).await.unwrap();
    assert_eq!(restored.counter().published(), Some(false));
    assert!(!restored.guard().is_initialized());
}

#[tokio::test]
async fn sqlite_obeys_the_continuation_repository_contract() {
    let root = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(root.path());
    contract(WorkflowContinuationRepositoryImpl::open(&path).unwrap()).await;
}

#[tokio::test]
async fn memory_obeys_the_continuation_repository_contract() {
    contract(WorkflowContinuationRepositoryImpl::in_memory()).await;
}
