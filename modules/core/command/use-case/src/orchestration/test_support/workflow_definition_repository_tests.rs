use super::*;

#[tokio::test]
async fn different_definition_streams_are_saved_and_revised_independently() {
    let mut repository = InMemoryWorkflowDefinitionRepository::empty();
    let (first, first_event) =
        WorkflowDefinition::define(definition_id(), &compiled(3), at()).unwrap();
    let (graph, grid, scopes) = content(3);
    let other_bundle = CompiledDefinition::compile(
        CompiledDefinitionId::parse("codex").unwrap(),
        graph,
        grid,
        scopes,
    )
    .0;
    let (second, second_event) = WorkflowDefinition::define(
        WorkflowDefinitionId::parse("codex").unwrap(),
        &other_bundle,
        at(),
    )
    .unwrap();

    repository.store(&first_event, &first).await.unwrap();
    repository.store(&second_event, &second).await.unwrap();
    let mut first = repository.find_by_id(first.id()).await.unwrap();
    let second = repository.find_by_id(second.id()).await.unwrap();
    let stale = first.clone();
    let event = first.redefine(&compiled(5), at()).unwrap();
    repository.store(&event, &first).await.unwrap();

    assert_eq!(
        repository.find_by_id(first.id()).await.unwrap().version(),
        2
    );
    assert_eq!(repository.find_by_id(second.id()).await.unwrap(), second);
    assert_eq!(second.version(), 1);
    assert!(matches!(
        repository.store(&event, &stale).await,
        Err(RepositoryError::Conflict {
            expected: 1,
            actual: 2
        })
    ));
    assert!(matches!(
        repository
            .find_by_id(&WorkflowDefinitionId::parse("kiro").unwrap())
            .await,
        Err(RepositoryError::NotFound { .. })
    ));
}
