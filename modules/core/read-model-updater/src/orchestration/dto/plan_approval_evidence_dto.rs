//! 計画承認の発行時点の根拠を保持する、この側が所有するDTO。
use super::DtoDecodeError;
use core_command_domain::orchestration::{
    CodeGenerationAuthority, IntentId, PlanApprovalEvidence, PlanTarget,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct PlanApprovalEvidenceDto {
    authority: AuthorityDto,
    fingerprint: String,
    questions_file: String,
    questions_sha256: String,
    prompt_sha256: String,
}
impl PlanApprovalEvidenceDto {
    pub(super) fn of(value: &PlanApprovalEvidence) -> Self {
        Self {
            authority: AuthorityDto::of(value.authority()),
            fingerprint: value.fingerprint().to_string(),
            questions_file: value.questions_file().to_string(),
            questions_sha256: value.questions_sha256().to_string(),
            prompt_sha256: value.prompt_sha256().to_string(),
        }
    }
    pub(super) fn to_domain(&self) -> Result<PlanApprovalEvidence, DtoDecodeError> {
        PlanApprovalEvidence::new(
            self.authority.to_domain()?,
            self.fingerprint.clone(),
            self.questions_file.clone(),
            self.questions_sha256.clone(),
            self.prompt_sha256.clone(),
        )
        .map_err(|_| DtoDecodeError::InvariantViolation)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AuthorityDto {
    unit: Option<String>,
    intent_id: String,
    directive_epoch: String,
    run_floor: String,
    source_floor: String,
    marker_revision: u64,
}
impl AuthorityDto {
    fn of(value: &CodeGenerationAuthority) -> Self {
        Self {
            unit: value.unit().map(str::to_string),
            intent_id: value.intent_id().to_string(),
            directive_epoch: value.directive_epoch().to_string(),
            run_floor: value.run_floor().to_string(),
            source_floor: value.source_floor().to_string(),
            marker_revision: value.marker_revision(),
        }
    }
    fn to_domain(&self) -> Result<CodeGenerationAuthority, DtoDecodeError> {
        let target = self
            .unit
            .as_ref()
            .map_or(Ok(PlanTarget::stage_level()), |unit| {
                PlanTarget::for_unit(unit).map_err(|_| DtoDecodeError::InvariantViolation)
            })?;
        let intent = IntentId::parse(&self.intent_id)
            .map_err(|_| DtoDecodeError::malformed("intent_id", &self.intent_id))?;
        CodeGenerationAuthority::new(
            &target,
            &intent,
            self.directive_epoch.clone(),
            self.run_floor.clone(),
            self.source_floor.clone(),
            self.marker_revision,
        )
        .map_err(|_| DtoDecodeError::InvariantViolation)
    }
}
