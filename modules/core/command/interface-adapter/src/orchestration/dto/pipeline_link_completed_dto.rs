//! Pipeline linkの保存DTO。読取側と書込側はそれぞれこの形式を所有する。
use super::dto_decode_error::DtoDecodeError;
use core_command_domain::orchestration::{
    IntentExecutionEventId, IntentExecutionId, PipelineHandoff, PipelineLinkCompleted,
    PipelineReceipt,
};
use serde::{Deserialize, Serialize};
/// 完了受領のワイヤ表現。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineLinkCompletedDto {
    id: String,
    aggregate_id: String,
    stage: String,
    link: String,
    repo: Option<String>,
    single: bool,
    position: usize,
    total: usize,
    handoff: Option<HandoffDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct HandoffDto {
    path: String,
    sha256: String,
    mtime_ms: String,
}
impl PipelineLinkCompletedDto {
    pub(super) fn of(event: &PipelineLinkCompleted) -> Self {
        let receipt = event.receipt();
        Self {
            id: event.id().as_str().into(),
            aggregate_id: event.aggregate_id().as_str().into(),
            stage: receipt.stage().into(),
            link: receipt.link().into(),
            repo: receipt.repo().map(str::to_string),
            single: receipt.is_single(),
            position: receipt.position(),
            total: receipt.total(),
            handoff: receipt.handoff().map(|h| HandoffDto {
                path: h.path().into(),
                sha256: h.sha256().into(),
                mtime_ms: h.mtime_ms().into(),
            }),
        }
    }
    pub(super) fn to_domain(&self) -> Result<PipelineLinkCompleted, DtoDecodeError> {
        let receipt = PipelineReceipt::new(
            self.stage.clone(),
            self.link.clone(),
            self.repo.clone(),
            self.single,
            self.position,
            self.total,
            self.handoff
                .as_ref()
                .map(|h| PipelineHandoff::new(h.path.clone(), h.sha256.clone(), h.mtime_ms.clone()))
                .transpose()
                .map_err(|_| DtoDecodeError::InvariantViolation)?,
        )
        .map_err(|_| DtoDecodeError::InvariantViolation)?;
        Ok(PipelineLinkCompleted::new(
            IntentExecutionEventId::parse(&self.id)
                .map_err(|_| DtoDecodeError::malformed("id", &self.id))?,
            IntentExecutionId::parse(&self.aggregate_id)
                .map_err(|_| DtoDecodeError::malformed("aggregate_id", &self.aggregate_id))?,
            receipt,
        ))
    }
}
