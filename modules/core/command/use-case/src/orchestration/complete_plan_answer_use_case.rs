//! 実行側の記録と投影の後に、共有承認への回答受領を完了する。
use super::{IntentExecutionRepository, PlanApprovalCommandError, PlanApprovalRuntimeRepository};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    IntentExecutionId, PlanApprovalOperationId, PlanApprovalRuntimeId,
};
use core_command_domain::workspace::SpaceName;
/// 判断は保存済み集約へ委ね、共有側の1イベントを保存する。
#[derive(Debug)]
pub struct CompletePlanAnswerUseCase<R, E> {
    approval_repository: R,
    execution_repository: E,
}
impl<R: PlanApprovalRuntimeRepository, E: IntentExecutionRepository>
    CompletePlanAnswerUseCase<R, E>
{
    /// 各保存先を注入する。
    #[must_use]
    pub const fn new(approval_repository: R, execution_repository: E) -> Self {
        Self {
            approval_repository,
            execution_repository,
        }
    }
    /// 保存済み監査を確認して受領を完了する。
    /// # Errors
    /// 未記録・対象不一致・保存失敗。
    pub async fn execute(
        &mut self,
        id: &PlanApprovalOperationId,
        space: &SpaceName,
        execution_id: &IntentExecutionId,
        at: DateTime<Utc>,
    ) -> Result<(), PlanApprovalCommandError> {
        let mut runtime = self
            .approval_repository
            .find_by_id(&PlanApprovalRuntimeId::Workspace)
            .await?;
        let execution = self.execution_repository.find_by_id(execution_id).await?;
        let event = runtime.complete_answer(id, space, &execution, at)?;
        self.approval_repository.store(&event, &runtime).await?;
        Ok(())
    }
}
