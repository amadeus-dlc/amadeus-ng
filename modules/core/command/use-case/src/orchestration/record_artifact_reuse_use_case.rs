//! 成果物再利用の受領の保存を指揮する更新UseCase。
use super::{IntentExecutionRepository, IntentRepository, WorkflowDefinitionRepository};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{ArtifactReuseReceipt, IntentExecutionId};
/// 定義と実行を再構成して集約へ受領を依頼し、単一イベントを保存する。
///
/// 記録専用の受領だが定義は要る — 集約は状態を動かさない代わりに、受領が名指した段が
/// 定義グラフに在ることだけを確かめる。
#[derive(Debug)]
pub struct RecordArtifactReuseUseCase<
    E: IntentExecutionRepository,
    I: IntentRepository,
    D: WorkflowDefinitionRepository,
> {
    executions: E,
    intents: I,
    definitions: D,
}
impl<E: IntentExecutionRepository, I: IntentRepository, D: WorkflowDefinitionRepository>
    RecordArtifactReuseUseCase<E, I, D>
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
        receipt: &ArtifactReuseReceipt,
        at: DateTime<Utc>,
    ) -> Result<(), super::ArtifactReuseCommandError> {
        match self.attempt(id, receipt, at).await {
            Err(super::ArtifactReuseCommandError::Execution(
                super::RepositoryError::Conflict { .. },
            )) => self.attempt(id, receipt, at).await,
            result => result,
        }
    }
    async fn attempt(
        &mut self,
        id: &IntentExecutionId,
        receipt: &ArtifactReuseReceipt,
        at: DateTime<Utc>,
    ) -> Result<(), super::ArtifactReuseCommandError> {
        use super::ArtifactReuseCommandError as E;
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
            .record_artifact_reuse(&intent, &definition, receipt.clone(), at)
            .map_err(E::Rejected)?;
        self.executions
            .store(&event, &execution)
            .await
            .map_err(E::Execution)
    }
}
