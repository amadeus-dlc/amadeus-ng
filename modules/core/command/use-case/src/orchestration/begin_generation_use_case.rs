//! 現在の実行による承認照合を経て、共有集約へ開始要求を保存する。
use super::{
    IntentExecutionRepository, IntentRepository, PlanApprovalCommandError,
    PlanApprovalRuntimeRepository,
};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    IntentExecutionId, PlanApprovalInput, PlanApprovalOperationId, PlanApprovalRuntimeId,
};
/// 開始判断はドメインへ委ね、表示結果を返さない。
#[derive(Debug)]
pub struct BeginGenerationUseCase<R, E, I> {
    approval_repository: R,
    execution_repository: E,
    intent_repository: I,
}
impl<R: PlanApprovalRuntimeRepository, E: IntentExecutionRepository, I: IntentRepository>
    BeginGenerationUseCase<R, E, I>
{
    /// 各所有者の保存ポートを注入する。
    #[must_use]
    pub const fn new(
        approval_repository: R,
        execution_repository: E,
        intent_repository: I,
    ) -> Self {
        Self {
            approval_repository,
            execution_repository,
            intent_repository,
        }
    }
    /// 開始操作を1イベントとして保存する。
    /// # Errors
    /// 承認不一致・ソース変化・保存失敗。
    pub async fn execute(
        &mut self,
        id: PlanApprovalOperationId,
        execution_id: &IntentExecutionId,
        input: &PlanApprovalInput,
        source_before: Option<&str>,
        at: DateTime<Utc>,
    ) -> Result<(), PlanApprovalCommandError> {
        let execution = self.execution_repository.find_by_id(execution_id).await?;
        let intent = self
            .intent_repository
            .find_by_id(execution.intent_id())
            .await?;
        let mut runtime = self
            .approval_repository
            .find_by_id(&PlanApprovalRuntimeId::Workspace)
            .await?;
        let approval = execution.code_generation_approval(&intent, input, &runtime);
        let event = runtime.request_generation(id, &approval, source_before, at)?;
        self.approval_repository.store(&event, &runtime).await?;
        Ok(())
    }
}
