//! 現在の文書と実際の選択を照合し、共有集約へ受領を記録する。
use super::{
    IntentExecutionRepository, IntentRepository, PlanAnswerRequest, PlanApprovalCommandError,
    PlanApprovalRuntimeRepository,
};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::PlanApprovalRuntimeId;
/// 検証は実行と承認の各集約が所有する。
#[derive(Debug)]
pub struct RecordPlanAnswerUseCase<R, E, I> {
    approval_repository: R,
    execution_repository: E,
    intent_repository: I,
}
impl<R: PlanApprovalRuntimeRepository, E: IntentExecutionRepository, I: IntentRepository>
    RecordPlanAnswerUseCase<R, E, I>
{
    /// 各集約の保存ポートを注入する。
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
    /// 受領候補を1イベントとして保存する。
    /// # Errors
    /// 文書・応答・ソース・保存の不一致。
    pub async fn execute(
        &mut self,
        request: &PlanAnswerRequest,
        at: DateTime<Utc>,
    ) -> Result<(), PlanApprovalCommandError> {
        let execution = self
            .execution_repository
            .find_by_id(request.origin().execution_id())
            .await?;
        let intent = self
            .intent_repository
            .find_by_id(execution.intent_id())
            .await?;
        let input = execution.verify_plan_answer(
            &intent,
            request.input(),
            request.origin(),
            request.stage(),
            request.session(),
            *request.choice(),
        )?;
        let mut runtime = self
            .approval_repository
            .find_by_id(&PlanApprovalRuntimeId::Workspace)
            .await?;
        let event = runtime.record_answer(request.operation_id().clone(), input, at)?;
        self.approval_repository.store(&event, &runtime).await?;
        Ok(())
    }
}
