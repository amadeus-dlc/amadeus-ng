//! 共有承認集約のスナップショット。書き手が所有するワイヤ形式。
use super::DtoDecodeError;
use super::plan_approval_evidence_dto::PlanApprovalEvidenceDto;
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    IntentExecutionId, PlanAppliedOperations, PlanApprovalOperationId, PlanApprovalRuntime,
    PlanApprovalRuntimeId, PlanChallenge, PlanChallengeOccurrence, PlanChallenges, PlanChoice,
    PlanHumanResponse, PlanInvalidation, PlanInvalidations, PlanSession,
};
use core_command_domain::workspace::SpaceName;
use serde::{Deserialize, Serialize};

/// 同一集約の基底として読み戻す全状態。版はストアの封筒が所有する。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanApprovalRuntimeDto {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    generations: Vec<super::plan_generation_dto::PlanGenerationDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    answers: Vec<super::plan_answer_dto::PlanAnswerDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    receipts: Vec<super::plan_receipt_dto::PlanReceiptDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pending_responses: Vec<super::plan_response_preparation_dto::PlanResponsePreparationDto>,
    id: String,
    challenges: Vec<ChallengeOccurrenceDto>,
    invalidations: Vec<InvalidationDto>,
    applied_operations: Vec<String>,
    seq_nr: usize,
    last_updated_at: DateTime<Utc>,
}
impl PlanApprovalRuntimeDto {
    /// ドメインの公開属性をワイヤへ写す。
    #[must_use]
    pub fn of(runtime: &PlanApprovalRuntime) -> Self {
        Self {
            generations: runtime
                .generations()
                .iter()
                .map(super::plan_generation_dto::PlanGenerationDto::of)
                .collect(),
            answers: runtime
                .answers()
                .iter()
                .map(super::plan_answer_dto::PlanAnswerDto::of)
                .collect(),
            receipts: runtime
                .receipts()
                .iter()
                .map(super::plan_receipt_dto::PlanReceiptDto::of)
                .collect(),
            pending_responses: runtime
                .pending_responses()
                .iter()
                .map(super::plan_response_preparation_dto::PlanResponsePreparationDto::of)
                .collect(),
            id: runtime.id().to_string(),
            challenges: runtime
                .challenges()
                .iter()
                .map(ChallengeOccurrenceDto::of)
                .collect(),
            invalidations: runtime
                .invalidations()
                .iter()
                .map(InvalidationDto::of)
                .collect(),
            applied_operations: runtime
                .applied_operations()
                .iter()
                .map(ToString::to_string)
                .collect(),
            seq_nr: runtime.seq_nr(),
            last_updated_at: runtime.last_updated_at(),
        }
    }
    /// 各値と集約の構築検査を経由して戻す。
    /// # Errors
    /// 値の不正、重複、集約の不変条件違反。
    pub fn to_domain(&self) -> Result<PlanApprovalRuntime, DtoDecodeError> {
        let challenges = self
            .challenges
            .iter()
            .map(ChallengeOccurrenceDto::to_domain)
            .collect::<Result<Vec<_>, _>>()?;
        let invalidations = self
            .invalidations
            .iter()
            .map(InvalidationDto::to_domain)
            .collect::<Result<Vec<_>, _>>()?;
        let applied = self
            .applied_operations
            .iter()
            .map(|id| operation_id(id))
            .collect::<Result<Vec<_>, _>>()?;
        PlanApprovalRuntime::new(
            core_command_domain::orchestration::PlanGenerations::new(
                self.generations
                    .iter()
                    .map(super::plan_generation_dto::PlanGenerationDto::to_domain)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .map_err(|_| DtoDecodeError::InvariantViolation)?,
            core_command_domain::orchestration::PlanAnswers::new(
                self.answers
                    .iter()
                    .map(super::plan_answer_dto::PlanAnswerDto::to_domain)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .map_err(|_| DtoDecodeError::InvariantViolation)?,
            core_command_domain::orchestration::PlanReceipts::new(
                self.receipts
                    .iter()
                    .map(super::plan_receipt_dto::PlanReceiptDto::to_domain)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .map_err(|_| DtoDecodeError::InvariantViolation)?,
            core_command_domain::orchestration::PlanPendingResponses::new(
                self.pending_responses
                    .iter()
                    .map(
                        super::plan_response_preparation_dto::PlanResponsePreparationDto::to_domain,
                    )
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .map_err(|_| DtoDecodeError::InvariantViolation)?,
            runtime_id(&self.id)?,
            PlanChallenges::new(challenges).map_err(|_| DtoDecodeError::InvariantViolation)?,
            PlanInvalidations::new(invalidations)
                .map_err(|_| DtoDecodeError::InvariantViolation)?,
            PlanAppliedOperations::new(applied).map_err(|_| DtoDecodeError::InvariantViolation)?,
            self.seq_nr,
            self.last_updated_at,
        )
        .map_err(|_| DtoDecodeError::InvariantViolation)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ChallengeOccurrenceDto {
    id: String,
    challenge: ChallengeDto,
    response: Option<HumanResponseDto>,
}
impl ChallengeOccurrenceDto {
    pub(super) fn of(value: &PlanChallengeOccurrence) -> Self {
        Self {
            id: value.id().to_string(),
            challenge: ChallengeDto::of(value.challenge()),
            response: value.response().map(HumanResponseDto::of),
        }
    }
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ChallengeDto {
    id: String,
    evidence: PlanApprovalEvidenceDto,
    session: String,
    options: [String; 2],
    require_exact: bool,
}
impl ChallengeDto {
    fn of(value: &PlanChallenge) -> Self {
        Self {
            id: value.id().to_string(),
            evidence: PlanApprovalEvidenceDto::of(value.evidence()),
            session: value.session().raw().to_string(),
            options: value.options().clone(),
            require_exact: value.require_exact(),
        }
    }
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct HumanResponseDto {
    id: String,
    occurrence_id: String,
    choice: String,
    response_sha256: String,
}
impl HumanResponseDto {
    fn of(value: &PlanHumanResponse) -> Self {
        Self {
            id: value.id().to_string(),
            occurrence_id: value.occurrence_id().to_string(),
            choice: value.choice().as_str().to_string(),
            response_sha256: value.response_sha256().to_string(),
        }
    }
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct InvalidationDto {
    id: String,
    space: String,
    execution_id: String,
}
impl InvalidationDto {
    pub(super) fn of(value: &PlanInvalidation) -> Self {
        Self {
            id: value.id().to_string(),
            space: value.space().as_str().to_string(),
            execution_id: value.execution_id().as_str().to_string(),
        }
    }
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
