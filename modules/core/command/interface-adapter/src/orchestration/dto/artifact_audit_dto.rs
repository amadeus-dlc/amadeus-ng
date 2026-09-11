//! ArtifactAuditの保存境界DTO。
use super::DtoDecodeError;
use chrono::{DateTime, Utc};
use core_command_domain::workspace::{
    ArtifactAudit, ArtifactAuditId, ArtifactAuditRecord, HookHealthTarget,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// ArtifactAuditの集約スナップショットの永続化DTO。
pub struct ArtifactAuditDto {
    id: String,
    target: String,
    seq_nr: usize,
    version: usize,
    last_file: String,
    last_tool: String,
    last_context: String,
    last_created: bool,
    last_at: DateTime<Utc>,
}
impl ArtifactAuditDto {
    pub(crate) fn of(v: &ArtifactAudit) -> Self {
        Self {
            id: v.id().to_string(),
            target: v.target().relative_directory(),
            seq_nr: v.seq_nr(),
            version: v.version(),
            last_file: v.last_file().into(),
            last_tool: v.last_tool().into(),
            last_context: v.last_context().into(),
            last_created: v.last_created(),
            last_at: v.last_at(),
        }
    }
    pub(crate) fn to_domain(&self) -> Result<ArtifactAudit, DtoDecodeError> {
        let id = ArtifactAuditId::parse(&self.id)
            .map_err(|_| DtoDecodeError::malformed("id", &self.id))?;
        let target = HookHealthTarget::parse(&self.target)
            .map_err(|_| DtoDecodeError::malformed("target", &self.target))?;
        ArtifactAudit::new(
            id,
            target,
            self.seq_nr,
            self.version,
            ArtifactAuditRecord::new(
                self.last_file.clone(),
                self.last_tool.clone(),
                self.last_context.clone(),
                self.last_created,
                self.last_at,
            ),
        )
        .map_err(|_| DtoDecodeError::InvariantViolation)
    }
}
