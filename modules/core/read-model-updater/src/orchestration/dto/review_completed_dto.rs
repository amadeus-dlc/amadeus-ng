//! ReviewCompletedの内容結合を含む永続化表現。
use super::{dto_decode_error::DtoDecodeError, review_completion_dto::ReviewCompletionDto};
use core_command_domain::orchestration::{
    IntentExecutionEventId, IntentExecutionId, ReviewCompleted,
};
use core_command_domain::workflow_definition::StageSlug;
use serde::{Deserialize, Serialize};
/// この保存側が所有するレビューイベントDTO。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewCompletedDto {
    id: String,
    aggregate_id: String,
    stage: String,
    reviewer: String,
    iteration: u32,
    verdict: String,
    evidence: ReviewCompletionDto,
}
impl ReviewCompletedDto {
    pub(super) fn of(value: &ReviewCompleted) -> Self {
        Self {
            id: value.id().as_str().into(),
            aggregate_id: value.aggregate_id().as_str().into(),
            stage: value.stage().as_str().into(),
            reviewer: value.reviewer().into(),
            iteration: value.iteration(),
            verdict: super::dto_vocabulary::review_verdict_spelling(value.verdict()).to_string(),
            evidence: ReviewCompletionDto::of(value.evidence()),
        }
    }
    pub(super) fn to_domain(&self) -> Result<ReviewCompleted, DtoDecodeError> {
        Ok(ReviewCompleted::new(
            IntentExecutionEventId::parse(&self.id)
                .map_err(|_| DtoDecodeError::malformed("id", &self.id))?,
            IntentExecutionId::parse(&self.aggregate_id)
                .map_err(|_| DtoDecodeError::malformed("aggregate_id", &self.aggregate_id))?,
            StageSlug::parse(&self.stage)
                .map_err(|_| DtoDecodeError::malformed("stage", &self.stage))?,
            self.reviewer.clone(),
            self.iteration,
            super::dto_vocabulary::review_verdict_of(&self.verdict, "verdict")?,
            self.evidence.to_domain()?,
        ))
    }
}
