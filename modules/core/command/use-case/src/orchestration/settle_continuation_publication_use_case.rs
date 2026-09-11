//! 指定した停止要求の公開IO成否を集約へ伝え、単一事実として確定する。
use super::{ContinuationCommandError, WorkflowContinuationRepository};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    ContinuationPublicationObservation, WorkflowContinuationId,
};

/// 選択判断を作り直さず、未確定操作だけを集約に照合させる。
pub struct SettleContinuationPublicationUseCase<R> {
    repository: R,
}
impl<R: WorkflowContinuationRepository> SettleContinuationPublicationUseCase<R> {
    /// 保存先を注入する。
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
    /// 公開成功/失敗を保存する。成功戻り値はunit。
    /// # Errors
    /// 対象・要求・順序の不一致、保存障害。
    pub async fn execute(
        mut self,
        id: &WorkflowContinuationId,
        observation: &ContinuationPublicationObservation,
        at: DateTime<Utc>,
    ) -> Result<(), ContinuationCommandError> {
        let mut aggregate = self.repository.find_by_id(id).await?;
        let event = aggregate.record_publication(observation, at)?;
        self.repository.store(&event, &aggregate).await?;
        Ok(())
    }
}
