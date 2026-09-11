//! 計画承認の監査質問が保持する根拠とセッション。
use super::{DtoDecodeError, plan_approval_evidence_dto::PlanApprovalEvidenceDto};
use core_command_domain::orchestration::{PlanDecisionEvidence, PlanSession};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct PlanDecisionEvidenceDto {
    evidence: PlanApprovalEvidenceDto,
    session: String,
}
impl PlanDecisionEvidenceDto {
    pub(super) fn of(value: &PlanDecisionEvidence) -> Self {
        Self {
            evidence: PlanApprovalEvidenceDto::of(value.evidence()),
            session: value.session().raw().to_string(),
        }
    }
    pub(super) fn to_domain(&self) -> Result<PlanDecisionEvidence, DtoDecodeError> {
        Ok(PlanDecisionEvidence::new(
            self.evidence.to_domain()?,
            PlanSession::new(self.session.clone())
                .map_err(|_| DtoDecodeError::malformed("plan_session", &self.session))?,
        ))
    }
}
