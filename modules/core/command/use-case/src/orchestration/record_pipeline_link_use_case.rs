//! Pipeline link受領の保存を指揮する更新UseCase。
use super::{IntentExecutionRepository, IntentRepository, WorkflowDefinitionRepository};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{IntentExecutionId, PipelineLinkRequest};
/// 定義と実行を再構成して集約へ受領を依頼し、単一イベントを保存する。
#[derive(Debug)]
pub struct RecordPipelineLinkUseCase<
    E: IntentExecutionRepository,
    I: IntentRepository,
    D: WorkflowDefinitionRepository,
> {
    executions: E,
    intents: I,
    definitions: D,
}
impl<E: IntentExecutionRepository, I: IntentRepository, D: WorkflowDefinitionRepository>
    RecordPipelineLinkUseCase<E, I, D>
{
    /// 保存と関連取得のポートを注入する。
    #[must_use]
    pub const fn new(executions: E, intents: I, definitions: D) -> Self {
        Self {
            executions,
            intents,
            definitions,
        }
    }
    /// 受領を保存する。表示用の値は返さない。
    /// # Errors
    /// 読取・保存の失敗、または集約が受領を拒否した場合。競合は同じ要求で1回だけ再試行する。
    pub async fn execute(
        &mut self,
        id: &IntentExecutionId,
        request: &PipelineLinkRequest,
        at: DateTime<Utc>,
    ) -> Result<(), super::PipelineLinkCommandError> {
        match self.attempt(id, request, at).await {
            Err(super::PipelineLinkCommandError::Execution(super::RepositoryError::Conflict {
                ..
            })) => self.attempt(id, request, at).await,
            result => result,
        }
    }
    async fn attempt(
        &mut self,
        id: &IntentExecutionId,
        request: &PipelineLinkRequest,
        at: DateTime<Utc>,
    ) -> Result<(), super::PipelineLinkCommandError> {
        use super::PipelineLinkCommandError as E;
        let mut execution = self.executions.find_by_id(id).await.map_err(E::Execution)?;
        let intent = self
            .intents
            .find_by_id(execution.intent_id())
            .await
            .map_err(E::Intent)?;
        let definition = self
            .definitions
            .find_by_id(intent.definition_id())
            .await
            .map_err(E::Definition)?;
        let event = execution
            .record_pipeline_link(&intent, &definition, request, at)
            .map_err(E::Rejected)?;
        self.executions
            .store(&event, &execution)
            .await
            .map_err(E::Execution)
    }
}
