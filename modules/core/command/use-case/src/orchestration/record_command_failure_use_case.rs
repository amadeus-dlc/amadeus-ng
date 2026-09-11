//! コマンド失敗を既存の実行へ記録する。
use super::{IntentExecutionRepository, InteractionCommandError, RepositoryError};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{CommandFailure, IntentExecutionId};

/// 失敗診断を単一イベントとして保存する更新ユースケース。
#[derive(Debug)]
pub struct RecordCommandFailureUseCase<R: IntentExecutionRepository> {
    repository: R,
}
impl<R: IntentExecutionRepository> RecordCommandFailureUseCase<R> {
    /// 対象実行の保存ポートを注入する。
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
    /// 同じ失敗を記録する。競合時は対象を固定して1回だけ再試行する。
    /// # Errors
    /// 読込み・保存・集約の採番失敗。
    pub async fn execute(
        &mut self,
        id: &IntentExecutionId,
        failure: &CommandFailure,
        at: DateTime<Utc>,
    ) -> Result<(), InteractionCommandError> {
        match self.attempt(id, failure, at).await {
            Err(InteractionCommandError::Repository(RepositoryError::Conflict { .. })) => {
                self.attempt(id, failure, at).await
            }
            result => result,
        }
    }
    async fn attempt(
        &mut self,
        id: &IntentExecutionId,
        failure: &CommandFailure,
        at: DateTime<Utc>,
    ) -> Result<(), InteractionCommandError> {
        let mut execution = self
            .repository
            .find_by_id(id)
            .await
            .map_err(InteractionCommandError::Repository)?;
        let event = execution
            .record_command_failure(failure.clone(), at)
            .map_err(InteractionCommandError::Command)?;
        self.repository
            .store(&event, &execution)
            .await
            .map_err(InteractionCommandError::Repository)
    }
}
