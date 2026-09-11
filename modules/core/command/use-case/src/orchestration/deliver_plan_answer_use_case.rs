//! 受領公開後のソースを再照合し、元の実行へ監査を配送する。
use super::{IntentExecutionRepository, PlanApprovalCommandError, PlanApprovalRuntimeRepository};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    IntentExecutionId, PlanAnswerDelivery, PlanApprovalOperationId, PlanApprovalRuntimeId,
    PlanRuntimeError,
};
use core_command_domain::workspace::SpaceName;
/// ソースの再照合と重複判定を集約へ委ねる。
#[derive(Debug)]
pub struct DeliverPlanAnswerUseCase<R, E> {
    approval_repository: R,
    execution_repository: E,
}
impl<R: PlanApprovalRuntimeRepository, E: IntentExecutionRepository>
    DeliverPlanAnswerUseCase<R, E>
{
    /// 各所有者の保存先を注入する。
    #[must_use]
    pub const fn new(approval_repository: R, execution_repository: E) -> Self {
        Self {
            approval_repository,
            execution_repository,
        }
    }
    /// 未監査の回答だけを配送する。認証失敗も取消しイベントを保存する。
    /// # Errors
    /// 対象不一致・認証中のソース変更・保存失敗。
    pub async fn execute(
        &mut self,
        id: &PlanApprovalOperationId,
        space: &SpaceName,
        execution_id: &IntentExecutionId,
        source: Option<&str>,
        at: DateTime<Utc>,
    ) -> Result<(), PlanApprovalCommandError> {
        let mut runtime = self
            .approval_repository
            .find_by_id(&PlanApprovalRuntimeId::Workspace)
            .await?;
        let mut execution = self.execution_repository.find_by_id(execution_id).await?;
        match runtime.answer_delivery(id, space, &execution, source)? {
            PlanAnswerDelivery::Recorded => (),
            PlanAnswerDelivery::RecordRequired => {
                let event = execution.record_plan_answer(&runtime, id, space, source, at)?;
                self.execution_repository.store(&event, &execution).await?;
            }
            PlanAnswerDelivery::RejectCertification => {
                let event = runtime.abort_answer(id, space, &execution, source, at)?;
                self.approval_repository.store(&event, &runtime).await?;
                return Err(PlanRuntimeError::ReceiptSourceChanged.into());
            }
        }
        Ok(())
    }
}
