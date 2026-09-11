//! 操作IDごとに追跡する実装開始と、その認証状態。
use super::{
    PlanApprovalError, PlanApprovalOperationId, PlanApprovalReceipt, PlanGenerationState,
    PlanGenerationStatus,
};
/// 共有集約に所属する開始操作。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanGeneration {
    id: PlanApprovalOperationId,
    receipt: PlanApprovalReceipt,
    state: PlanGenerationState,
}
impl PlanGeneration {
    /// 所属する開始操作の確定イベントを適用する。
    /// # Errors
    /// 別操作のイベント、または照合待ちでない状態。
    pub(super) fn apply_certification(
        &mut self,
        event: &super::PlanGenerationCertified,
    ) -> Result<(), super::PlanRuntimeError> {
        if event.operation_id() != &self.id {
            return Err(super::PlanRuntimeError::InvalidState);
        }
        if self.state != PlanGenerationState::Pending {
            return Err(super::PlanRuntimeError::NoPendingGeneration);
        }
        self.state = PlanGenerationState::Active;
        Ok(())
    }
    /// 所属する開始操作の失効イベントを適用する。
    /// # Errors
    /// 別操作のイベント、または照合待ちでない状態。
    pub(super) fn apply_revocation(
        &mut self,
        event: &super::PlanGenerationRevoked,
    ) -> Result<(), super::PlanRuntimeError> {
        if event.operation_id() != &self.id {
            return Err(super::PlanRuntimeError::InvalidState);
        }
        if self.state != PlanGenerationState::Pending {
            return Err(super::PlanRuntimeError::NoPendingGeneration);
        }
        self.state = PlanGenerationState::Revoked;
        Ok(())
    }

    /// 開始用の受領と状態を束ねる。
    /// # Errors
    /// 受領がgenerationではない場合。
    pub fn new(
        id: PlanApprovalOperationId,
        receipt: PlanApprovalReceipt,
        state: PlanGenerationState,
    ) -> Result<Self, PlanApprovalError> {
        if receipt.status() != PlanGenerationStatus::Generation {
            return Err(PlanApprovalError::new(
                "generation operation requires a generation receipt",
            ));
        }
        Ok(Self { id, receipt, state })
    }
    /// 開始操作のID。
    #[must_use]
    pub const fn id(&self) -> &PlanApprovalOperationId {
        &self.id
    }
    /// 開始へ結び付けた受領。
    #[must_use]
    pub const fn receipt(&self) -> &PlanApprovalReceipt {
        &self.receipt
    }
    /// 認証状態。
    #[must_use]
    pub const fn state(&self) -> PlanGenerationState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::*;
    fn pending() -> PlanGeneration {
        let authority = CodeGenerationAuthority::new(
            &PlanTarget::stage_level(),
            &IntentId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
            format!("sha256:{}", "a".repeat(64)),
            "unstarted#0".to_string(),
            "b".repeat(64),
            1,
        )
        .unwrap();
        let evidence = PlanApprovalEvidence::new(
            authority,
            format!("sha256:{}", "c".repeat(64)),
            "questions.md".to_string(),
            "d".repeat(64),
            "e".repeat(64),
        )
        .unwrap();
        let receipt = PlanApprovalReceipt::new(
            PlanDecisionEvidence::new(evidence, PlanSession::new("s".to_string()).unwrap()),
            format!("sha256:{}", "f".repeat(64)),
            "b".repeat(64),
            PlanGenerationStatus::Generation,
        )
        .unwrap();
        PlanGeneration::new(
            PlanApprovalOperationId::generate(),
            receipt,
            PlanGenerationState::Pending,
        )
        .unwrap()
    }
    #[test]
    fn recorded_generation_events_require_the_matching_pending_operation() {
        let mut generation = pending();
        let before = generation.clone();
        let foreign = PlanGenerationCertified::new(
            PlanApprovalEventId::generate(),
            PlanApprovalRuntimeId::Workspace,
            PlanApprovalOperationId::generate(),
        );
        assert!(generation.apply_certification(&foreign).is_err());
        assert_eq!(generation, before);
        let event = PlanGenerationCertified::new(
            PlanApprovalEventId::generate(),
            PlanApprovalRuntimeId::Workspace,
            generation.id().clone(),
        );
        generation.apply_certification(&event).unwrap();
        assert_eq!(generation.state(), PlanGenerationState::Active);
        let active = generation.clone();
        assert!(generation.apply_certification(&event).is_err());
        assert_eq!(generation, active);
        let revoked = PlanGenerationRevoked::new(
            PlanApprovalEventId::generate(),
            PlanApprovalRuntimeId::Workspace,
            generation.id().clone(),
        );
        assert!(generation.apply_revocation(&revoked).is_err());
        assert_eq!(generation, active);
        let mut generation = pending();
        let revoked = PlanGenerationRevoked::new(
            PlanApprovalEventId::generate(),
            PlanApprovalRuntimeId::Workspace,
            generation.id().clone(),
        );
        generation.apply_revocation(&revoked).unwrap();
        assert_eq!(generation.state(), PlanGenerationState::Revoked);
        assert!(generation.apply_revocation(&revoked).is_err());
    }
}
