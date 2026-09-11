//! 回答を集約へ渡し、その結果を保存する更新ユースケース。
use super::{IntentExecutionRepository, InteractionCommandError, RepositoryError};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{AnswerId, AnswerRequest, IntentExecutionId};
/// 回答の受理判断を集約に委ねる。
#[derive(Debug)]
pub struct RecordAnswerUseCase<R: IntentExecutionRepository> {
    repository: R,
}
impl<R: IntentExecutionRepository> RecordAnswerUseCase<R> {
    /// 保存ポートを注入する。
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
    /// 指定IDの回答を保存する。結果はRMU経由のQueryで取得する。
    ///
    /// # Errors
    /// 集約の拒否または読込み・保存失敗。競合時も回答IDと要求を保持する。
    pub async fn execute(
        &mut self,
        id: &IntentExecutionId,
        answer_id: &AnswerId,
        request: &AnswerRequest,
        at: DateTime<Utc>,
    ) -> Result<(), InteractionCommandError> {
        match self.attempt(id, answer_id, request, at).await {
            Err(InteractionCommandError::Repository(RepositoryError::Conflict { .. })) => {
                self.attempt(id, answer_id, request, at).await
            }
            result => result,
        }
    }
    async fn attempt(
        &mut self,
        id: &IntentExecutionId,
        answer_id: &AnswerId,
        request: &AnswerRequest,
        at: DateTime<Utc>,
    ) -> Result<(), InteractionCommandError> {
        let mut aggregate = self
            .repository
            .find_by_id(id)
            .await
            .map_err(InteractionCommandError::Repository)?;
        let event = aggregate
            .record_answer(answer_id.clone(), request, at)
            .map_err(InteractionCommandError::Answer)?;
        self.repository
            .store(&event, &aggregate)
            .await
            .map_err(InteractionCommandError::Repository)
    }
}
