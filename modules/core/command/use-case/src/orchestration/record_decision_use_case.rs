//! 質問の提示を集約へ渡し、その事実を保存する更新ユースケース。
use super::interaction_command_error::InteractionCommandError;
use super::{IntentExecutionRepository, RepositoryError};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{DecisionPrompt, IntentExecutionId};

/// 通常の質問提示の更新経路。
#[derive(Debug)]
pub struct RecordDecisionUseCase<R: IntentExecutionRepository> {
    repository: R,
}
impl<R: IntentExecutionRepository> RecordDecisionUseCase<R> {
    /// 保存ポートを注入する。
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
    /// 質問を記録する。成功時の表示材料は返さない。
    ///
    /// # Errors
    /// 集約の拒否、読込み・保存失敗。楽観競合は同じ要求で1回だけ再試行する。
    pub async fn execute(
        &mut self,
        id: &IntentExecutionId,
        prompt: &DecisionPrompt,
        at: DateTime<Utc>,
    ) -> Result<(), InteractionCommandError> {
        match self.attempt(id, prompt, at).await {
            Err(InteractionCommandError::Repository(RepositoryError::Conflict { .. })) => {
                self.attempt(id, prompt, at).await
            }
            result => result,
        }
    }
    async fn attempt(
        &mut self,
        id: &IntentExecutionId,
        prompt: &DecisionPrompt,
        at: DateTime<Utc>,
    ) -> Result<(), InteractionCommandError> {
        let mut aggregate = self
            .repository
            .find_by_id(id)
            .await
            .map_err(InteractionCommandError::Repository)?;
        let event = aggregate
            .record_decision(prompt.clone(), at)
            .map_err(InteractionCommandError::Command)?;
        self.repository
            .store(&event, &aggregate)
            .await
            .map_err(InteractionCommandError::Repository)
    }
}
