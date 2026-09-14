//! PreCompactの文脈失効を、共有承認と実行の保存事実に結び付ける。
use super::{IntentExecutionRepository, PlanApprovalCommandError, PlanApprovalRuntimeRepository};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    DirectiveContextInvalidation, IntentExecutionId, PlanApprovalOperationId,
    PlanApprovalRuntimeId, PlanInvalidation,
};
use core_command_domain::workspace::SpaceName;
/// 共有排他を持つ呼出側から使う。途中失敗は既存の失効回復操作が解決する。
#[derive(Debug)]
pub struct InvalidateDirectiveContextUseCase<R, E> {
    approval_repository: R,
    execution_repository: E,
}
impl<R: PlanApprovalRuntimeRepository, E: IntentExecutionRepository>
    InvalidateDirectiveContextUseCase<R, E>
{
    /// 両集約の保存ポートを注入する。
    #[must_use]
    pub const fn new(approval_repository: R, execution_repository: E) -> Self {
        Self {
            approval_repository,
            execution_repository,
        }
    }
    /// 一致しない発火元では何も保存しない。
    /// # Errors
    /// 世代枯渇、共有操作の拒否、読取または保存の失敗。
    pub async fn execute(
        &mut self,
        execution_id: &IntentExecutionId,
        space: &SpaceName,
        operation: &PlanApprovalOperationId,
        request: &DirectiveContextInvalidation,
        at: DateTime<Utc>,
    ) -> Result<(), PlanApprovalCommandError> {
        let mut execution = self.execution_repository.find_by_id(execution_id).await?;
        if !execution.matches_directive_context(request) {
            return Ok(());
        }
        let invalidated = execution.invalidate_directive_context(operation, request, at)?;
        let mut runtime = self
            .approval_repository
            .find_by_id(&PlanApprovalRuntimeId::Workspace)
            .await?;
        let prepared = runtime.prepare_invalidation(
            PlanInvalidation::new(operation.clone(), space.clone(), execution_id.clone()),
            at,
        )?;
        self.approval_repository.store(&prepared, &runtime).await?;
        self.execution_repository
            .store(&invalidated, &execution)
            .await?;
        let mut runtime = self
            .approval_repository
            .find_by_id(&PlanApprovalRuntimeId::Workspace)
            .await?;
        let resolved = runtime.resolve_for_publication(operation, space, &execution, at)?;
        self.approval_repository.store(&resolved, &runtime).await?;
        Ok(())
    }
}
