//! 指示文脈失効イベントの永続化表現。
use super::{active_directive_dto::ActiveDirectiveDto, dto_decode_error::DtoDecodeError};
use core_command_domain::orchestration::{
    DirectiveContextInvalidated, IntentExecutionEventId, IntentExecutionId,
};
use serde::{Deserialize, Serialize};
/// 書込み側/読込み側がそれぞれ所有するDTO。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectiveContextInvalidatedDto {
    id: String,
    aggregate_id: String,
    directive: ActiveDirectiveDto,
}
impl DirectiveContextInvalidatedDto {
    pub(super) fn of(event: &DirectiveContextInvalidated) -> Self {
        Self {
            id: event.id().as_str().to_string(),
            aggregate_id: event.aggregate_id().as_str().to_string(),
            directive: ActiveDirectiveDto::of(event.directive()),
        }
    }
    pub(super) fn to_domain(&self) -> Result<DirectiveContextInvalidated, DtoDecodeError> {
        Ok(DirectiveContextInvalidated::new(
            IntentExecutionEventId::parse(&self.id)
                .map_err(|_| DtoDecodeError::malformed("id", &self.id))?,
            IntentExecutionId::parse(&self.aggregate_id)
                .map_err(|_| DtoDecodeError::malformed("aggregate_id", &self.aggregate_id))?,
            self.directive.to_domain()?,
        ))
    }
}
