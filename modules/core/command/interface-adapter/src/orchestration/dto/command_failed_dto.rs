//! コマンド失敗の永続化表現。ドメインとは独立したDTO。
use super::dto_decode_error::DtoDecodeError;
use core_command_domain::orchestration::{
    CommandFailed, CommandFailure, IntentExecutionEventId, IntentExecutionId,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandFailedDto {
    id: String,
    aggregate_id: String,
    tool: String,
    command: String,
    error: String,
}
impl CommandFailedDto {
    pub(super) fn of(event: &CommandFailed) -> Self {
        Self {
            id: event.id().as_str().to_string(),
            aggregate_id: event.aggregate_id().as_str().to_string(),
            tool: event.failure().tool().to_string(),
            command: event.failure().command().to_string(),
            error: event.failure().error().to_string(),
        }
    }
    pub(super) fn to_domain(&self) -> Result<CommandFailed, DtoDecodeError> {
        Ok(CommandFailed::new(
            IntentExecutionEventId::parse(&self.id)
                .map_err(|_| DtoDecodeError::malformed("id", &self.id))?,
            IntentExecutionId::parse(&self.aggregate_id)
                .map_err(|_| DtoDecodeError::malformed("aggregate_id", &self.aggregate_id))?,
            CommandFailure::new(self.tool.clone(), self.command.clone(), self.error.clone()),
        ))
    }
}
