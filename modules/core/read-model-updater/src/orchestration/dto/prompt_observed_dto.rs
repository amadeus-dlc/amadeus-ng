//! フックが観測した応答の保存表現。
use super::dto_decode_error::DtoDecodeError;
use core_command_domain::orchestration::{
    IntentExecutionEventId, IntentExecutionId, PromptObserved,
};
use serde::{Deserialize, Serialize};
/// PromptObservedの書込み/読込み契約。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptObservedDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    approval_observation_id: Option<String>,
    id: String,
    aggregate_id: String,
    session: String,
    response: String,
    unattended: bool,
}
impl PromptObservedDto {
    pub(super) fn of(event: &PromptObserved) -> Self {
        Self {
            approval_observation_id: event.approval_observation_id().map(ToString::to_string),
            id: event.id().as_str().to_string(),
            aggregate_id: event.aggregate_id().as_str().to_string(),
            session: event.session().to_string(),
            response: event.response().to_string(),
            unattended: event.unattended(),
        }
    }
    pub(super) fn to_domain(&self) -> Result<PromptObserved, DtoDecodeError> {
        Ok(PromptObserved::new(
            IntentExecutionEventId::parse(&self.id)
                .map_err(|_| DtoDecodeError::malformed("id", &self.id))?,
            IntentExecutionId::parse(&self.aggregate_id)
                .map_err(|_| DtoDecodeError::malformed("aggregate_id", &self.aggregate_id))?,
            &self.session,
            &self.response,
            self.unattended,
        )
        .with_approval_observation(
            self.approval_observation_id
                .as_ref()
                .map(|id| {
                    core_command_domain::orchestration::PlanApprovalOperationId::parse(id)
                        .map_err(|_| DtoDecodeError::malformed("approval_observation_id", id))
                })
                .transpose()?,
        ))
    }
}
