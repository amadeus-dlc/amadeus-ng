//! 受領から監査完了までの回答を写す、この側が所有する保存形式。
use super::{
    DtoDecodeError, plan_answer_input_dto::PlanAnswerInputDto,
    plan_approval_runtime_dto::operation_id, plan_receipt_dto::PlanReceiptDto,
};
use core_command_domain::orchestration::{PlanAnswer, PlanAnswerState};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct PlanAnswerDto {
    id: String,
    input: PlanAnswerInputDto,
    occurrence_id: String,
    challenge_id: String,
    response_id: String,
    receipt: Option<PlanReceiptDto>,
    state: String,
    error: Option<String>,
}
impl PlanAnswerDto {
    pub(super) fn to_domain(&self) -> Result<PlanAnswer, DtoDecodeError> {
        let state = match (self.state.as_str(), &self.error) {
            ("pending", None) => PlanAnswerState::Pending,
            ("recorded", None) => PlanAnswerState::Recorded,
            ("aborted", Some(error)) => PlanAnswerState::Aborted(error.clone()),
            _ => return Err(DtoDecodeError::InvariantViolation),
        };
        PlanAnswer::new(
            operation_id(&self.id)?,
            self.input.to_domain()?,
            operation_id(&self.occurrence_id)?,
            self.challenge_id.clone(),
            operation_id(&self.response_id)?,
            self.receipt
                .as_ref()
                .map(PlanReceiptDto::to_domain)
                .transpose()?,
            state,
        )
        .map_err(|_| DtoDecodeError::InvariantViolation)
    }
}
