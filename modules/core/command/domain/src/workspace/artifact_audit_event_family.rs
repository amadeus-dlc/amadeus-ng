//! ArtifactAuditイベントの公開族。
use super::{ArtifactAuditEventId, ArtifactAuditId, ArtifactSaved, ArtifactWriteObservation};
/// 保存観測イベント族。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactAuditEvent {
    /// 成果物保存を観測した。
    Saved(ArtifactSaved),
}
impl ArtifactAuditEvent {
    /// イベントID。
    #[must_use]
    pub const fn id(&self) -> &ArtifactAuditEventId {
        match self {
            Self::Saved(v) => v.id(),
        }
    }
    /// 集約ID。
    #[must_use]
    pub const fn aggregate_id(&self) -> &ArtifactAuditId {
        match self {
            Self::Saved(v) => v.aggregate_id(),
        }
    }
    /// 観測材料。
    #[must_use]
    pub const fn observation(&self) -> &ArtifactWriteObservation {
        match self {
            Self::Saved(v) => v.observation(),
        }
    }
}
