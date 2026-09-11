//! 計画承認の質問を検証し、各所有者へ事実を保存する。
use super::{
    IntentExecutionRepository, IntentRepository, PlanApprovalCommandError,
    PlanApprovalRuntimeRepository, PlanDecisionRequest,
};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    IntentExecutionId, PlanApprovalRuntimeId, PlanChallenge, PlanDecisionEvidence,
};
/// 監査はspace側の実行が所有し、承認提示はワークスペース全体の集約が所有する。
#[derive(Debug)]
pub struct RecordPlanDecisionUseCase<R, E, I> {
    approval_repository: R,
    execution_repository: E,
    intent_repository: I,
}
impl<R: PlanApprovalRuntimeRepository, E: IntentExecutionRepository, I: IntentRepository>
    RecordPlanDecisionUseCase<R, E, I>
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
    /// 質問と提示を記録する。成功戻り値には表示用材料を含めない。
    /// # Errors
    /// 対象・内容・発行の拒否、または各ストアの読取/保存失敗。
    pub async fn execute(
        &mut self,
        execution_id: &IntentExecutionId,
        request: &PlanDecisionRequest,
        at: DateTime<Utc>,
    ) -> Result<(), PlanApprovalCommandError> {
        let mut execution = self.execution_repository.find_by_id(execution_id).await?;
        let intent = self
            .intent_repository
            .find_for_execution(&execution)
            .await?;
        let evidence = execution.plan_approval_evidence(&intent, request.input(), "")?;
        let prompt = request
            .prompt()
            .clone()
            .with_plan_approval(PlanDecisionEvidence::new(
                evidence.clone(),
                request.session().clone(),
            ));
        let decision = execution.record_decision(prompt, at)?;
        self.execution_repository
            .store(&decision, &execution)
            .await?;
        let challenge = PlanChallenge::from_prompt(
            evidence,
            request.session().clone(),
            request.prompt(),
            request.exact(),
        )?;
        let mut approval = self
            .approval_repository
            .find_by_id(&PlanApprovalRuntimeId::Workspace)
            .await?;
        let offered = approval.issue_challenge(request.operation_id().clone(), challenge, at)?;
        self.approval_repository.store(&offered, &approval).await?;
        Ok(())
    }
}
