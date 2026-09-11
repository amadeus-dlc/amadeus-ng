//! 共有承認イベントのRMU側ワイヤ形式。
use super::DtoDecodeError;
use super::plan_approval_runtime_dto::{
    ChallengeOccurrenceDto, InvalidationDto, choice, operation_id, runtime_id, session,
};
use super::plan_response_preparation_dto::PlanResponsePreparationDto;
use core_command_domain::orchestration::{
    PlanApprovalEvent, PlanApprovalEventId, PlanChallengeIssued, PlanInvalidationPrepared,
    PlanInvalidationResolved, PlanResponseObserved, PlanRuntimeCreated,
};
use serde::Deserialize;
/// 封筒の通番・時刻とは別に、イベント自身の識別子を保持する。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct PlanApprovalEventDto {
    id: String,
    aggregate_id: String,
    payload: Payload,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", content = "value")]
enum Payload {
    Created,
    GenerationRequested(Box<super::plan_generation_dto::PlanGenerationDto>),
    GenerationCertified {
        operation_id: String,
    },
    GenerationRevoked {
        operation_id: String,
    },
    AnswerRecorded(Box<super::plan_answer_dto::PlanAnswerDto>),
    AnswerCompleted {
        operation_id: String,
    },
    AnswerAborted {
        operation_id: String,
    },
    ResponsePrepared(PlanResponsePreparationDto),
    ChallengeIssued(Box<ChallengeOccurrenceDto>),
    ResponseObserved {
        observation_id: String,
        occurrence_id: String,
        session: String,
        response_sha256: String,
        choice: Option<String>,
    },
    InvalidationPrepared(InvalidationDto),
    InvalidationResolved {
        operation_id: String,
        published: bool,
    },
}
impl PlanApprovalEventDto {
    /// 記録済みの事実を各値の検査経由で復号する。
    /// # Errors
    /// 識別子や値の表記・内容が不正な場合。
    pub(crate) fn to_domain(&self) -> Result<PlanApprovalEvent, DtoDecodeError> {
        let id = PlanApprovalEventId::parse(&self.id)
            .map_err(|_| DtoDecodeError::malformed("event_id", &self.id))?;
        let aggregate = runtime_id(&self.aggregate_id)?;
        Ok(match &self.payload {
            Payload::GenerationRequested(value) => {
                PlanApprovalEvent::GenerationRequested(Box::new(
                    core_command_domain::orchestration::PlanGenerationRequested::new(
                        id,
                        aggregate,
                        value.to_domain()?,
                    ),
                ))
            }
            Payload::GenerationCertified { operation_id: raw } => {
                PlanApprovalEvent::GenerationCertified(
                    core_command_domain::orchestration::PlanGenerationCertified::new(
                        id,
                        aggregate,
                        operation_id(raw)?,
                    ),
                )
            }
            Payload::GenerationRevoked { operation_id: raw } => {
                PlanApprovalEvent::GenerationRevoked(
                    core_command_domain::orchestration::PlanGenerationRevoked::new(
                        id,
                        aggregate,
                        operation_id(raw)?,
                    ),
                )
            }
            Payload::ResponsePrepared(value) => PlanApprovalEvent::ResponsePrepared(
                core_command_domain::orchestration::PlanResponsePrepared::new(
                    id,
                    aggregate,
                    value.to_domain()?,
                ),
            ),
            Payload::AnswerRecorded(value) => PlanApprovalEvent::AnswerRecorded(Box::new(
                core_command_domain::orchestration::PlanAnswerRecorded::new(
                    id,
                    aggregate,
                    value.to_domain()?,
                ),
            )),
            Payload::AnswerCompleted { operation_id: raw } => PlanApprovalEvent::AnswerCompleted(
                core_command_domain::orchestration::PlanAnswerCompleted::new(
                    id,
                    aggregate,
                    operation_id(raw)?,
                ),
            ),
            Payload::AnswerAborted { operation_id: raw } => PlanApprovalEvent::AnswerAborted(
                core_command_domain::orchestration::PlanAnswerAborted::new(
                    id,
                    aggregate,
                    operation_id(raw)?,
                ),
            ),
            Payload::Created => PlanApprovalEvent::Created(PlanRuntimeCreated::new(id, aggregate)),
            Payload::ChallengeIssued(value) => PlanApprovalEvent::ChallengeIssued(Box::new(
                PlanChallengeIssued::new(id, aggregate, value.to_domain()?),
            )),
            Payload::ResponseObserved {
                observation_id,
                occurrence_id,
                session: raw_session,
                response_sha256,
                choice: raw_choice,
            } => PlanApprovalEvent::ResponseObserved(PlanResponseObserved::new(
                id,
                aggregate,
                operation_id(observation_id)?,
                operation_id(occurrence_id)?,
                session(raw_session)?,
                response_sha256.clone(),
                raw_choice.as_deref().map(choice).transpose()?,
            )),
            Payload::InvalidationPrepared(value) => PlanApprovalEvent::InvalidationPrepared(
                PlanInvalidationPrepared::new(id, aggregate, value.to_domain()?),
            ),
            Payload::InvalidationResolved {
                operation_id: raw,
                published,
            } => PlanApprovalEvent::InvalidationResolved(PlanInvalidationResolved::new(
                id,
                aggregate,
                operation_id(raw)?,
                *published,
            )),
        })
    }
}
