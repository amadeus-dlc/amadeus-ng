//! 成果物再利用の保存DTO。読取側と書込側はそれぞれこの形式を所有する。
use super::dto_decode_error::DtoDecodeError;
use core_command_domain::orchestration::{
    ArtifactReuseReceipt, ArtifactReused, IntentExecutionEventId, IntentExecutionId,
};
use serde::{Deserialize, Serialize};
/// 再利用受領のワイヤ表現。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactReusedDto {
    id: String,
    aggregate_id: String,
    stage: String,
    decision: String,
    artifacts: String,
    repo: Option<String>,
    single: bool,
}
impl ArtifactReusedDto {
    pub(super) fn of(event: &ArtifactReused) -> Self {
        let receipt = event.receipt();
        Self {
            id: event.id().as_str().into(),
            aggregate_id: event.aggregate_id().as_str().into(),
            stage: receipt.stage().into(),
            decision: receipt.decision().into(),
            artifacts: receipt.artifacts().into(),
            repo: receipt.repo().map(str::to_string),
            single: receipt.is_single(),
        }
    }
    pub(super) fn to_domain(&self) -> Result<ArtifactReused, DtoDecodeError> {
        let receipt = ArtifactReuseReceipt::new(
            self.stage.clone(),
            self.decision.clone(),
            self.artifacts.clone(),
            self.repo.clone(),
            self.single,
        )
        .map_err(|_| DtoDecodeError::InvariantViolation)?;
        Ok(ArtifactReused::new(
            IntentExecutionEventId::parse(&self.id)
                .map_err(|_| DtoDecodeError::malformed("id", &self.id))?,
            IntentExecutionId::parse(&self.aggregate_id)
                .map_err(|_| DtoDecodeError::malformed("aggregate_id", &self.aggregate_id))?,
            receipt,
        ))
    }
}
