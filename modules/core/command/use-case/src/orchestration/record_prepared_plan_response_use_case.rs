//! 観測時の応答を、実行側へ重複なく配送する。
use super::{IntentExecutionRepository, PlanApprovalCommandError, PlanApprovalRuntimeRepository};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    IntentExecutionId, PlanApprovalOperationId, PlanApprovalRuntimeId, PlanResponseDelivery,
};
use core_command_domain::workspace::SpaceName;
/// 配送済みかの判断を集約へ委ね、各ストアを順に更新する。
#[derive(Debug)]
pub struct RecordPreparedPlanResponseUseCase<R, E> {
    approval_repository: R,
    execution_repository: E,
}
impl<R: PlanApprovalRuntimeRepository, E: IntentExecutionRepository>
    RecordPreparedPlanResponseUseCase<R, E>
{
    /// それぞれの集約の保存先を注入する。
    #[must_use]
    pub const fn new(approval_repository: R, execution_repository: E) -> Self {
        Self {
            approval_repository,
            execution_repository,
        }
    }
    /// 準備時の対象へ配送し、その事実を確認して受領を確定する。
    /// # Errors
    /// 対象不一致・記録不備・各ストアの失敗。
    pub async fn execute(
        &mut self,
        id: &PlanApprovalOperationId,
        space: &SpaceName,
        execution_id: &IntentExecutionId,
        at: DateTime<Utc>,
    ) -> Result<(), PlanApprovalCommandError> {
        let runtime = self
            .approval_repository
            .find_by_id(&PlanApprovalRuntimeId::Workspace)
            .await?;
        let mut execution = self.execution_repository.find_by_id(execution_id).await?;
        match runtime.response_delivery(id, space, &execution)? {
            PlanResponseDelivery::RecordRequired => {
                let event = execution.record_prepared_response(&runtime, id, at)?;
                self.execution_repository.store(&event, &execution).await?;
            }
            PlanResponseDelivery::Recorded => (),
        }
        Ok(())
    }
}
