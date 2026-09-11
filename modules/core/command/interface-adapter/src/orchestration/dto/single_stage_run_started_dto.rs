//! 単独pipeline開始の保存DTO。
use super::dto_decode_error::DtoDecodeError;
use core_command_domain::{
    orchestration::{IntentExecutionEventId, IntentExecutionId, SingleStageRunStarted},
    workflow_definition::StageSlug,
};
use serde::{Deserialize, Serialize};
/// 開始事実のワイヤ表現。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SingleStageRunStartedDto {
    id: String,
    aggregate_id: String,
    stage: String,
}
impl SingleStageRunStartedDto {
    pub(super) fn of(e: &SingleStageRunStarted) -> Self {
        Self {
            id: e.id().as_str().into(),
            aggregate_id: e.aggregate_id().as_str().into(),
            stage: e.stage().as_str().into(),
        }
    }
    pub(super) fn to_domain(&self) -> Result<SingleStageRunStarted, DtoDecodeError> {
        Ok(SingleStageRunStarted::new(
            IntentExecutionEventId::parse(&self.id)
                .map_err(|_| DtoDecodeError::InvariantViolation)?,
            IntentExecutionId::parse(&self.aggregate_id)
                .map_err(|_| DtoDecodeError::InvariantViolation)?,
            StageSlug::parse(&self.stage).map_err(|_| DtoDecodeError::InvariantViolation)?,
        ))
    }
}
