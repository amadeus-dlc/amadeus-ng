//! 実際の提示と応答に一致した、保護された計画承認の受領。
use super::{PlanApprovalError, PlanDecisionEvidence, PlanGenerationStatus};
/// 文書と実装開始時のソース基準へ結び付けた受領値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanApprovalReceipt {
    decision: PlanDecisionEvidence,
    challenge_id: String,
    certified_source: String,
    status: PlanGenerationStatus,
}
impl PlanApprovalReceipt {
    /// 受領の値を、その指紋とソース基準を検査して束ねる。
    /// # Errors
    /// 指紋が不正、または認証したソースと発行時の基準が異なる場合。
    pub fn new(
        decision: PlanDecisionEvidence,
        challenge_id: String,
        certified_source: String,
        status: PlanGenerationStatus,
    ) -> Result<Self, PlanApprovalError> {
        let hex = |value: &str| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        };
        if !challenge_id.strip_prefix("sha256:").is_some_and(hex)
            || !hex(&certified_source)
            || certified_source != decision.evidence().authority().source_floor()
        {
            return Err(PlanApprovalError::new(
                "invalid protected Plan Approval receipt",
            ));
        }
        Ok(Self {
            decision,
            challenge_id,
            certified_source,
            status,
        })
    }
    /// 同じ承認認証を指すか。開始前後の状態だけは比較から分ける。
    #[must_use]
    pub fn same_certification(&self, other: &Self) -> bool {
        self.decision == other.decision
            && self.challenge_id == other.challenge_id
            && self.certified_source == other.certified_source
    }
    pub(super) fn for_generation(&self) -> Result<Self, PlanApprovalError> {
        Self::new(
            self.decision.clone(),
            self.challenge_id.clone(),
            self.certified_source.clone(),
            PlanGenerationStatus::Generation,
        )
    }
    /// 対象と発行エポックから決まる受領のキー。
    #[must_use]
    pub fn key(&self) -> String {
        core_infrastructure::hash::sha256_hex(
            format!(
                "{}\n{}",
                self.decision.evidence().authority().target_id(),
                self.decision.evidence().authority().directive_epoch()
            )
            .as_bytes(),
        )
    }
    /// 照合した文書とセッション。
    #[must_use]
    pub const fn decision(&self) -> &PlanDecisionEvidence {
        &self.decision
    }
    /// 提示された選択肢の識別子。
    #[must_use]
    pub fn challenge_id(&self) -> &str {
        &self.challenge_id
    }
    /// 受領時に認証したソース。
    #[must_use]
    pub fn certified_source(&self) -> &str {
        &self.certified_source
    }
    /// 実装開始前後の状態。
    #[must_use]
    pub const fn status(&self) -> PlanGenerationStatus {
        self.status
    }
}
