//! ReviewRequestedの内容結合を含む永続化表現。
use super::{dto_decode_error::DtoDecodeError, review_binding_dto::ReviewBindingDto};
use core_command_domain::orchestration::{
    IntentExecutionEventId, IntentExecutionId, ReviewRequested,
};
use core_command_domain::workflow_definition::StageSlug;
use serde::{Deserialize, Serialize};
/// この保存側が所有するレビューイベントDTO。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewRequestedDto {
    id: String,
    aggregate_id: String,
    stage: String,
    reviewer: String,
    iteration: u32,
    retry: bool,
    evidence: ReviewBindingDto,
}
impl ReviewRequestedDto {
    pub(super) fn of(value: &ReviewRequested) -> Self {
        Self {
            id: value.id().as_str().into(),
            aggregate_id: value.aggregate_id().as_str().into(),
            stage: value.stage().as_str().into(),
            reviewer: value.reviewer().into(),
            iteration: value.iteration(),
            retry: value.is_retry(),
            evidence: ReviewBindingDto::of(value.evidence()),
        }
    }
    pub(super) fn to_domain(&self) -> Result<ReviewRequested, DtoDecodeError> {
        Ok(ReviewRequested::new(
            IntentExecutionEventId::parse(&self.id)
                .map_err(|_| DtoDecodeError::malformed("id", &self.id))?,
            IntentExecutionId::parse(&self.aggregate_id)
                .map_err(|_| DtoDecodeError::malformed("aggregate_id", &self.aggregate_id))?,
            StageSlug::parse(&self.stage)
                .map_err(|_| DtoDecodeError::malformed("stage", &self.stage))?,
            self.reviewer.clone(),
            self.iteration,
            self.retry,
            self.evidence.to_domain()?,
        ))
    }
}
