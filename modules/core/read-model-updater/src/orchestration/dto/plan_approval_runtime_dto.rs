//! 共有承認イベントの値を読む、RMU独自の復号型。
use super::DtoDecodeError;
use super::plan_approval_evidence_dto::PlanApprovalEvidenceDto;
use core_command_domain::orchestration::{
    IntentExecutionId, PlanApprovalOperationId, PlanApprovalRuntimeId, PlanChallenge,
    PlanChallengeOccurrence, PlanChoice, PlanHumanResponse, PlanInvalidation, PlanSession,
};
use core_command_domain::workspace::SpaceName;
use serde::Deserialize;
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct ChallengeOccurrenceDto {
    id: String,
    challenge: ChallengeDto,
    response: Option<HumanResponseDto>,
}
impl ChallengeOccurrenceDto {
    pub(super) fn to_domain(&self) -> Result<PlanChallengeOccurrence, DtoDecodeError> {
        let occurrence =
            PlanChallengeOccurrence::new(operation_id(&self.id)?, self.challenge.to_domain()?);
        self.response
            .as_ref()
            .map_or(Ok(occurrence.clone()), |response| {
                occurrence
                    .with_response(response.to_domain()?)
                    .map_err(|_| DtoDecodeError::InvariantViolation)
            })
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct ChallengeDto {
    id: String,
    evidence: PlanApprovalEvidenceDto,
    session: String,
    options: [String; 2],
    require_exact: bool,
}
impl ChallengeDto {
    fn to_domain(&self) -> Result<PlanChallenge, DtoDecodeError> {
        let challenge = PlanChallenge::issue(
            self.evidence.to_domain()?,
            session(&self.session)?,
            self.options.clone(),
            self.require_exact,
        );
        if challenge.id() != self.id {
            return Err(DtoDecodeError::InvariantViolation);
        }
        Ok(challenge)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct HumanResponseDto {
    id: String,
    occurrence_id: String,
    choice: String,
    response_sha256: String,
}
impl HumanResponseDto {
    fn to_domain(&self) -> Result<PlanHumanResponse, DtoDecodeError> {
        PlanHumanResponse::new(
            operation_id(&self.id)?,
            operation_id(&self.occurrence_id)?,
            choice(&self.choice)?,
            self.response_sha256.clone(),
        )
        .map_err(|_| DtoDecodeError::InvariantViolation)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct InvalidationDto {
    id: String,
    space: String,
    execution_id: String,
}
impl InvalidationDto {
    pub(super) fn to_domain(&self) -> Result<PlanInvalidation, DtoDecodeError> {
        Ok(PlanInvalidation::new(
            operation_id(&self.id)?,
            SpaceName::parse(&self.space)
                .map_err(|_| DtoDecodeError::malformed("space", &self.space))?,
            IntentExecutionId::parse(&self.execution_id)
                .map_err(|_| DtoDecodeError::malformed("execution_id", &self.execution_id))?,
        ))
    }
}
pub(super) fn runtime_id(value: &str) -> Result<PlanApprovalRuntimeId, DtoDecodeError> {
    if value == "workspace" {
        Ok(PlanApprovalRuntimeId::Workspace)
    } else {
        Err(DtoDecodeError::malformed("runtime_id", value))
    }
}
pub(super) fn operation_id(value: &str) -> Result<PlanApprovalOperationId, DtoDecodeError> {
    PlanApprovalOperationId::parse(value)
        .map_err(|_| DtoDecodeError::malformed("operation_id", value))
}
pub(super) fn session(value: &str) -> Result<PlanSession, DtoDecodeError> {
    PlanSession::new(value.to_string()).map_err(|_| DtoDecodeError::malformed("session", value))
}
pub(super) fn choice(value: &str) -> Result<PlanChoice, DtoDecodeError> {
    match value {
        "Approve Plan" => Ok(PlanChoice::ApprovePlan),
        "Request Changes" => Ok(PlanChoice::RequestChanges),
        _ => Err(DtoDecodeError::malformed("choice", value)),
    }
}
