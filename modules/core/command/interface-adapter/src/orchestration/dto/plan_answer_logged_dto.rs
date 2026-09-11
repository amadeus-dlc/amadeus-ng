//! 元の実行への計画回答監査を写す、この側が所有する保存形式。
use super::{
    DtoDecodeError, plan_answer_input_dto::PlanAnswerInputDto,
    plan_approval_runtime_dto::operation_id,
};
use core_command_domain::orchestration::{
    IntentExecutionEventId, IntentExecutionId, PlanAnswerLogged,
};
use serde::{Deserialize, Serialize};
/// 計画回答の監査イベントの保存表現。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanAnswerLoggedDto {
    id: String,
    aggregate_id: String,
    operation_id: String,
    input: PlanAnswerInputDto,
}
impl PlanAnswerLoggedDto {
    pub(super) fn of(value: &PlanAnswerLogged) -> Self {
        Self {
            id: value.id().to_string(),
            aggregate_id: value.aggregate_id().to_string(),
            operation_id: value.operation_id().to_string(),
            input: PlanAnswerInputDto::of(value.input()),
        }
    }
    pub(super) fn to_domain(&self) -> Result<PlanAnswerLogged, DtoDecodeError> {
        Ok(PlanAnswerLogged::new(
            IntentExecutionEventId::parse(&self.id)
                .map_err(|_| DtoDecodeError::malformed("event_id", &self.id))?,
            IntentExecutionId::parse(&self.aggregate_id)
                .map_err(|_| DtoDecodeError::malformed("aggregate_id", &self.aggregate_id))?,
            operation_id(&self.operation_id)?,
            self.input.to_domain()?,
        ))
    }
}
