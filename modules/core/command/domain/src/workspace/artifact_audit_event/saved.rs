//! 成果物監査集約の保存事実。
use crate::workspace::{ArtifactAuditEventId, ArtifactAuditId, ArtifactWriteObservation};
#[derive(Debug, Clone, PartialEq, Eq)]
/// 完成した成果物監査のドメイン型。
pub struct ArtifactSaved {
    id: ArtifactAuditEventId,
    aggregate_id: ArtifactAuditId,
    observation: ArtifactWriteObservation,
}
impl ArtifactSaved {
    #[must_use]
    /// 検査済みの値を返す。
    pub const fn new(
        id: ArtifactAuditEventId,
        aggregate_id: ArtifactAuditId,
        observation: ArtifactWriteObservation,
    ) -> Self {
        Self {
            id,
            aggregate_id,
            observation,
        }
    }
    #[must_use]
    /// 検査済みの値を返す。
    pub const fn id(&self) -> &ArtifactAuditEventId {
        &self.id
    }
    #[must_use]
    /// 検査済みの値を返す。
    pub const fn aggregate_id(&self) -> &ArtifactAuditId {
        &self.aggregate_id
    }
    #[must_use]
    /// 検査済みの値を返す。
    pub const fn observation(&self) -> &ArtifactWriteObservation {
        &self.observation
    }
}
