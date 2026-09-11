//! 人間応答を観測時の発行回へ固定する準備の、この側が所有するDTO。
use super::DtoDecodeError;
use super::plan_approval_runtime_dto::{choice, operation_id, session};
use core_command_domain::orchestration::{
    IntentExecutionId, PlanApprovalOrigin, PlanResponsePreparation,
};
use core_command_domain::workspace::SpaceName;
use serde::Deserialize;
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct PlanResponsePreparationDto {
    id: String,
    space: String,
    execution_id: String,
    occurrence_id: String,
    session: String,
    response: String,
    choice: Option<String>,
}
impl PlanResponsePreparationDto {
    pub(super) fn to_domain(&self) -> Result<PlanResponsePreparation, DtoDecodeError> {
        Ok(PlanResponsePreparation::new(
            operation_id(&self.id)?,
            PlanApprovalOrigin::new(
                SpaceName::parse(&self.space)
                    .map_err(|_| DtoDecodeError::malformed("space", &self.space))?,
                IntentExecutionId::parse(&self.execution_id)
                    .map_err(|_| DtoDecodeError::malformed("execution_id", &self.execution_id))?,
            ),
            operation_id(&self.occurrence_id)?,
            session(&self.session)?,
            self.response.clone(),
            self.choice.as_deref().map(choice).transpose()?,
        ))
    }
}
