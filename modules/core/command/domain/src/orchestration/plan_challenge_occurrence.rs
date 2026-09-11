//! 内容が同じでも別の提示を区別する発行回。
use super::{PlanApprovalOperationId, PlanChallenge};
/// 共有承認集約に属する提示のローカルエンティティ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanChallengeOccurrence {
    response: Option<super::PlanHumanResponse>,
    id: PlanApprovalOperationId,
    challenge: PlanChallenge,
}
impl PlanChallengeOccurrence {
    /// 同じ発行回の応答を持つ状態へ再構成する。
    /// # Errors
    /// 応答の対象がこの発行回と異なる場合。
    pub fn with_response(
        mut self,
        response: super::PlanHumanResponse,
    ) -> Result<Self, super::PlanApprovalError> {
        if response.occurrence_id() != &self.id {
            return Err(super::PlanApprovalError::new(
                "response belongs to another challenge issuance",
            ));
        }
        self.response = Some(response);
        Ok(self)
    }

    /// この発行回に結び付いた最後の有効応答。
    #[must_use]
    pub const fn response(&self) -> Option<&super::PlanHumanResponse> {
        self.response.as_ref()
    }
    pub(super) fn observe(&mut self, response: super::PlanHumanResponse) {
        self.response = Some(response);
    }

    /// 提示した回と検証済み内容を結ぶ。
    #[must_use]
    pub const fn new(id: PlanApprovalOperationId, challenge: PlanChallenge) -> Self {
        Self {
            response: None,
            id,
            challenge,
        }
    }
    /// 提示の発行回。内容から作る公開challengeIdとは別である。
    #[must_use]
    pub const fn id(&self) -> &PlanApprovalOperationId {
        &self.id
    }
    /// 提示した内容。
    #[must_use]
    pub const fn challenge(&self) -> &PlanChallenge {
        &self.challenge
    }
}
