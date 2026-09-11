//! 人間応答の発行回と原文を、配送前に保存する。
use super::{PlanApprovalCommandError, PlanApprovalRuntimeRepository};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    PlanApprovalOperationId, PlanApprovalOrigin, PlanApprovalRuntimeId, PlanSession,
};
/// 応答の解釈は集約が行い、UseCaseはその1イベントを保存する。
#[derive(Debug)]
pub struct PreparePlanResponseUseCase<R> {
    repository: R,
}
impl<R: PlanApprovalRuntimeRepository> PreparePlanResponseUseCase<R> {
    /// 共有承認の保存先を注入する。
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
    /// 観測時点の提示へ応答を固定する。
    /// # Errors
    /// 該当提示なし・先行操作待ち・I/O・競合。
    pub async fn execute(
        &mut self,
        id: PlanApprovalOperationId,
        origin: PlanApprovalOrigin,
        session: PlanSession,
        response: &str,
        at: DateTime<Utc>,
    ) -> Result<(), PlanApprovalCommandError> {
        let mut runtime = self
            .repository
            .find_by_id(&PlanApprovalRuntimeId::Workspace)
            .await?;
        let event = runtime.prepare_response(id, origin, session, response, at)?;
        self.repository.store(&event, &runtime).await?;
        Ok(())
    }
}
