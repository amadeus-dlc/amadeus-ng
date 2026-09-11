//! 観測した人間の選択に一致して受領した、計画回答。
use super::{
    PlanAnswerInput, PlanAnswerState, PlanApprovalError, PlanApprovalOperationId,
    PlanApprovalReceipt, PlanChoice,
};
/// 共有承認集約が所有し、同じ操作IDで監査完了まで追跡するローカルエンティティ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanAnswer {
    id: PlanApprovalOperationId,
    input: PlanAnswerInput,
    occurrence_id: PlanApprovalOperationId,
    challenge_id: String,
    response_id: PlanApprovalOperationId,
    receipt: Option<PlanApprovalReceipt>,
    state: PlanAnswerState,
}
impl PlanAnswer {
    /// 選択と受領値の対応を検査して完全な状態を組む。
    /// # Errors
    /// 選択と受領の有無・内容が一致しない場合。
    pub fn new(
        id: PlanApprovalOperationId,
        input: PlanAnswerInput,
        occurrence_id: PlanApprovalOperationId,
        challenge_id: String,
        response_id: PlanApprovalOperationId,
        receipt: Option<PlanApprovalReceipt>,
        state: PlanAnswerState,
    ) -> Result<Self, PlanApprovalError> {
        let valid = match (input.choice(), receipt.as_ref()) {
            (PlanChoice::ApprovePlan, Some(receipt)) => {
                receipt.decision() == input.decision() && receipt.challenge_id() == challenge_id
            }
            (PlanChoice::RequestChanges, None) => true,
            _ => false,
        };
        if !valid {
            return Err(PlanApprovalError::new(
                "plan answer and receipt do not match",
            ));
        }
        Ok(Self {
            id,
            input,
            occurrence_id,
            challenge_id,
            response_id,
            receipt,
            state,
        })
    }
    /// 回答操作の識別子。
    #[must_use]
    pub const fn id(&self) -> &PlanApprovalOperationId {
        &self.id
    }
    /// 回答時点の材料。
    #[must_use]
    pub const fn input(&self) -> &PlanAnswerInput {
        &self.input
    }
    /// 回答した提示の発行回。
    #[must_use]
    pub const fn occurrence_id(&self) -> &PlanApprovalOperationId {
        &self.occurrence_id
    }
    /// 提示内容の識別子。
    #[must_use]
    pub fn challenge_id(&self) -> &str {
        &self.challenge_id
    }
    /// 使った人間応答の観測操作。
    #[must_use]
    pub const fn response_id(&self) -> &PlanApprovalOperationId {
        &self.response_id
    }
    /// 認証した受領値。修正要求ではNone。
    #[must_use]
    pub const fn receipt(&self) -> Option<&PlanApprovalReceipt> {
        self.receipt.as_ref()
    }
    /// 監査配送までの状態。
    #[must_use]
    pub const fn state(&self) -> &PlanAnswerState {
        &self.state
    }
    pub(super) fn record(&mut self) {
        self.state = PlanAnswerState::Recorded;
    }
    pub(super) fn abort(&mut self, message: String) {
        self.state = PlanAnswerState::Aborted(message);
    }
}
