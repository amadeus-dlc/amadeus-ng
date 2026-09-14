//! 定義のID検索・保存と、テスト専用の観測・異常応答を分離する。

use std::{cell::Cell, collections::HashMap};

use core_command_domain::workflow_definition::{
    WorkflowDefinition, WorkflowDefinitionEvent, WorkflowDefinitionId,
};

use super::{RepositoryError, WorkflowDefinitionRepository};

/// IDごとに定義を保持するユースケース単体テスト用のRepository。
#[derive(Debug)]
pub(crate) struct InMemoryWorkflowDefinitionRepository {
    stored: HashMap<WorkflowDefinitionId, WorkflowDefinition>,
}

impl InMemoryWorkflowDefinitionRepository {
    pub(crate) const fn new(stored: HashMap<WorkflowDefinitionId, WorkflowDefinition>) -> Self {
        Self { stored }
    }

    pub(crate) fn empty() -> Self {
        Self::new(HashMap::new())
    }

    pub(crate) fn holding(definition: WorkflowDefinition) -> Self {
        Self::new(HashMap::from([(
            definition.id().clone(),
            definition.with_version(1),
        )]))
    }
}

impl WorkflowDefinitionRepository for InMemoryWorkflowDefinitionRepository {
    async fn find_by_id(
        &self,
        id: &WorkflowDefinitionId,
    ) -> Result<WorkflowDefinition, RepositoryError<WorkflowDefinitionId>> {
        self.stored
            .get(id)
            .cloned()
            .ok_or_else(|| RepositoryError::NotFound { id: id.clone() })
    }

    async fn store(
        &mut self,
        _event: &WorkflowDefinitionEvent,
        definition: &WorkflowDefinition,
    ) -> Result<(), RepositoryError<WorkflowDefinitionId>> {
        let expected = definition.version();
        let actual = self
            .stored
            .get(definition.id())
            .map_or(0, WorkflowDefinition::version);
        if expected != actual {
            return Err(RepositoryError::Conflict { expected, actual });
        }
        self.stored.insert(
            definition.id().clone(),
            definition.clone().with_version(actual + 1),
        );
        Ok(())
    }
}

/// 読取回数と成功した書込みを観測する。通常のRepositoryに計測状態を持たせない。
pub(crate) struct WorkflowDefinitionRepositorySpy<R> {
    inner: R,
    lookups: Cell<usize>,
    committed: Vec<WorkflowDefinitionEvent>,
}

impl<R> WorkflowDefinitionRepositorySpy<R> {
    pub(crate) const fn new(inner: R) -> Self {
        Self {
            inner,
            lookups: Cell::new(0),
            committed: Vec::new(),
        }
    }

    pub(crate) fn lookups(&self) -> usize {
        self.lookups.get()
    }

    pub(crate) fn committed(&self) -> &[WorkflowDefinitionEvent] {
        &self.committed
    }
}

impl<R: WorkflowDefinitionRepository> WorkflowDefinitionRepository
    for WorkflowDefinitionRepositorySpy<R>
{
    async fn find_by_id(
        &self,
        id: &WorkflowDefinitionId,
    ) -> Result<WorkflowDefinition, RepositoryError<WorkflowDefinitionId>> {
        self.lookups.set(self.lookups.get() + 1);
        self.inner.find_by_id(id).await
    }

    async fn store(
        &mut self,
        event: &WorkflowDefinitionEvent,
        definition: &WorkflowDefinition,
    ) -> Result<(), RepositoryError<WorkflowDefinitionId>> {
        self.inner.store(event, definition).await?;
        self.committed.push(event.clone());
        Ok(())
    }
}

/// 検索IDに反する定義を返す異常応答。取り違えの拒否とエラー伝播の検査専用。
pub(crate) struct MisdirectedWorkflowDefinitionRepository {
    definition: WorkflowDefinition,
}

impl MisdirectedWorkflowDefinitionRepository {
    pub(crate) const fn new(definition: WorkflowDefinition) -> Self {
        Self { definition }
    }
}

impl WorkflowDefinitionRepository for MisdirectedWorkflowDefinitionRepository {
    async fn find_by_id(
        &self,
        _id: &WorkflowDefinitionId,
    ) -> Result<WorkflowDefinition, RepositoryError<WorkflowDefinitionId>> {
        Ok(self.definition.clone())
    }

    #[expect(
        clippy::panic,
        reason = "テスト専用の異常応答で、禁止した書込みが呼ばれたら検査を失敗させる"
    )]
    async fn store(
        &mut self,
        _event: &WorkflowDefinitionEvent,
        _definition: &WorkflowDefinition,
    ) -> Result<(), RepositoryError<WorkflowDefinitionId>> {
        panic!("取り違えた定義を書き込んではならない")
    }
}

/// 読取りが破損で失敗する異常応答。
pub(crate) struct UnreadableWorkflowDefinitionRepository;

impl WorkflowDefinitionRepository for UnreadableWorkflowDefinitionRepository {
    async fn find_by_id(
        &self,
        id: &WorkflowDefinitionId,
    ) -> Result<WorkflowDefinition, RepositoryError<WorkflowDefinitionId>> {
        Err(RepositoryError::Corrupt {
            id: id.clone(),
            seq_nr: Some(1),
            source: Box::new(std::io::Error::other("journal row is unreadable")),
        })
    }

    #[expect(
        clippy::panic,
        reason = "テスト専用の異常応答で、読取失敗後の書込みを検出する"
    )]
    async fn store(
        &mut self,
        _event: &WorkflowDefinitionEvent,
        _definition: &WorkflowDefinition,
    ) -> Result<(), RepositoryError<WorkflowDefinitionId>> {
        panic!("読取りに失敗した定義を書き込んではならない")
    }
}

/// 読取り後に他の書き手が先に更新した場合の競合応答。
pub(crate) struct ConflictingWorkflowDefinitionRepository {
    inner: InMemoryWorkflowDefinitionRepository,
}

impl ConflictingWorkflowDefinitionRepository {
    pub(crate) fn new(definition: WorkflowDefinition) -> Self {
        Self {
            inner: InMemoryWorkflowDefinitionRepository::holding(definition),
        }
    }
}

impl WorkflowDefinitionRepository for ConflictingWorkflowDefinitionRepository {
    async fn find_by_id(
        &self,
        id: &WorkflowDefinitionId,
    ) -> Result<WorkflowDefinition, RepositoryError<WorkflowDefinitionId>> {
        self.inner.find_by_id(id).await
    }

    async fn store(
        &mut self,
        _event: &WorkflowDefinitionEvent,
        definition: &WorkflowDefinition,
    ) -> Result<(), RepositoryError<WorkflowDefinitionId>> {
        Err(RepositoryError::Conflict {
            expected: definition.version(),
            actual: definition.version() + 1,
        })
    }
}
