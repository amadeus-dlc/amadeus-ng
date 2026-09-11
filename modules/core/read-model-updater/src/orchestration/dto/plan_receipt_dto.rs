//! 保護された受領を写す、この側が所有する保存形式。
use super::{
    DtoDecodeError, plan_approval_evidence_dto::PlanApprovalEvidenceDto,
    plan_approval_runtime_dto::session,
};
use core_command_domain::orchestration::{
    PlanApprovalReceipt, PlanDecisionEvidence, PlanGenerationStatus,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct PlanReceiptDto {
    evidence: PlanApprovalEvidenceDto,
    session: String,
    challenge_id: String,
    certified_source: String,
    status: String,
}
impl PlanReceiptDto {
    pub(super) fn to_domain(&self) -> Result<PlanApprovalReceipt, DtoDecodeError> {
        PlanApprovalReceipt::new(
            PlanDecisionEvidence::new(self.evidence.to_domain()?, session(&self.session)?),
            self.challenge_id.clone(),
            self.certified_source.clone(),
            match self.status.as_str() {
                "approved" => PlanGenerationStatus::Approved,
                "generation" => PlanGenerationStatus::Generation,
                _ => return Err(DtoDecodeError::malformed("receipt_status", &self.status)),
            },
        )
        .map_err(|_| DtoDecodeError::InvariantViolation)
    }
}
