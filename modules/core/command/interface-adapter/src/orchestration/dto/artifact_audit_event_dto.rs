//! ArtifactAuditイベントの保存境界DTO。
use super::DtoDecodeError;
use core_command_domain::workspace::{
    ArtifactAuditEvent, ArtifactAuditEvent::Saved, ArtifactAuditEventId, ArtifactAuditId,
    ArtifactSaved, ArtifactWriteObservation, HookHealthTarget,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// ArtifactAuditのイベントの永続化DTO。
pub struct ArtifactAuditEventDto {
    id: String,
    aggregate_id: String,
    target: String,
    tool: String,
    file: String,
    context: String,
    created: bool,
}
impl ArtifactAuditEventDto {
    pub(crate) fn of(v: &ArtifactAuditEvent) -> Self {
        let o = v.observation();
        Self {
            id: v.id().to_string(),
            aggregate_id: v.aggregate_id().to_string(),
            target: o.target().relative_directory(),
            tool: o.tool().into(),
            file: o.file().into(),
            context: o.context().into(),
            created: o.created(),
        }
    }
    pub(crate) fn to_domain(&self) -> Result<ArtifactAuditEvent, DtoDecodeError> {
        let id = ArtifactAuditEventId::parse(&self.id)
            .map_err(|_| DtoDecodeError::malformed("id", &self.id))?;
        let aid = ArtifactAuditId::parse(&self.aggregate_id)
            .map_err(|_| DtoDecodeError::malformed("aggregate_id", &self.aggregate_id))?;
        let target = HookHealthTarget::parse(&self.target)
            .map_err(|_| DtoDecodeError::malformed("target", &self.target))?;
        let o = ArtifactWriteObservation::new(
            target,
            self.tool.clone(),
            self.file.clone(),
            self.context.clone(),
            self.created,
        );
        Ok(Saved(ArtifactSaved::new(id, aid, o)))
    }
}
