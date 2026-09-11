//! 元の実行が保存した発行事実から、未完了の承認失効を回復する。
use super::{IntentExecutionRepository, PlanApprovalCommandError, PlanApprovalRuntimeRepository};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    IntentExecutionId, PlanApprovalOperationId, PlanApprovalRuntimeId,
};
use core_command_domain::workspace::SpaceName;
/// 各集約をそれぞれのRepositoryで取得し、失効判断を共有承認の所有者へ委ねる。
#[derive(Debug)]
pub struct RecoverPlanInvalidationUseCase<R, E> {
    approval_repository: R,
    execution_repository: E,
}
impl<R: PlanApprovalRuntimeRepository, E: IntentExecutionRepository>
    RecoverPlanInvalidationUseCase<R, E>
{
    /// 元の発行先と承認状態のRepositoryを注入する。
    #[must_use]
    pub const fn new(approval_repository: R, execution_repository: E) -> Self {
        Self {
            approval_repository,
            execution_repository,
        }
    }
    /// 識別された未完了操作を回復する。操作全体の排他は呼出境界が保持する。
    /// # Errors
    /// 対象不一致、集約の拒否、読取・保存の失敗。
    pub async fn execute(
        &mut self,
        operation: &PlanApprovalOperationId,
        space: &SpaceName,
        execution_id: &IntentExecutionId,
        at: DateTime<Utc>,
    ) -> Result<(), PlanApprovalCommandError> {
        let mut runtime = self
            .approval_repository
            .find_by_id(&PlanApprovalRuntimeId::Workspace)
            .await?;
        let execution = self.execution_repository.find_by_id(execution_id).await?;
        let event = runtime.resolve_for_publication(operation, space, &execution, at)?;
        self.approval_repository.store(&event, &runtime).await?;
        Ok(())
    }
}
