//! 実装開始の要求と認証状態を写す、この側が所有するワイヤ形式。
use super::{
    DtoDecodeError, plan_approval_runtime_dto::operation_id, plan_receipt_dto::PlanReceiptDto,
};
use core_command_domain::orchestration::{PlanGeneration, PlanGenerationState};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct PlanGenerationDto {
    id: String,
    receipt: PlanReceiptDto,
    state: String,
}
impl PlanGenerationDto {
    pub(super) fn to_domain(&self) -> Result<PlanGeneration, DtoDecodeError> {
        PlanGeneration::new(
            operation_id(&self.id)?,
            self.receipt.to_domain()?,
            match self.state.as_str() {
                "pending" => PlanGenerationState::Pending,
                "active" => PlanGenerationState::Active,
                "revoked" => PlanGenerationState::Revoked,
                _ => return Err(DtoDecodeError::malformed("generation_state", &self.state)),
            },
        )
        .map_err(|_| DtoDecodeError::InvariantViolation)
    }
}
