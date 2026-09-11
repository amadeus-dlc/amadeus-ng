//! TaskUpdateの同期を既存作業へ保存する。
use super::{IntentExecutionRepository, TaskSynchronizationError};
use chrono::{DateTime, Utc};
use core_command_domain::{orchestration::IntentExecutionId, workflow_definition::StageSlug};
/// TaskUpdate専用の更新UseCase。成功時に表示用値を返さない。
#[derive(Debug)]
pub struct SynchronizeTaskUseCase<R> {
    repository: R,
}
impl<R: IntentExecutionRepository> SynchronizeTaskUseCase<R> {
    /// 実行の保存ポートを注入する。
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
    /// 指定stageへの同期事実を1件保存する。
    /// # Errors
    /// 未知stage、通番上限、Repositoryの読取り・保存失敗。
    pub async fn execute(
        &mut self,
        id: &IntentExecutionId,
        stage: &StageSlug,
        at: DateTime<Utc>,
    ) -> Result<(), TaskSynchronizationError> {
        let mut execution = self
            .repository
            .find_by_id(id)
            .await
            .map_err(TaskSynchronizationError::Repository)?;
        let event = execution
            .synchronize_task(stage, at)
            .map_err(TaskSynchronizationError::Command)?;
        self.repository
            .store(&event, &execution)
            .await
            .map_err(TaskSynchronizationError::Repository)
    }
}
