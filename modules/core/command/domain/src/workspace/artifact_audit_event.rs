//! ArtifactAuditイベントの公開族。

// 変種ペイロードは 1 ファイル 1 公開型で本ファイル同名のサブツリーに置き、ここで連鎖
// 再輸出する (所有サブツリーのファサード — `coding-rules/module-visibility.md`
// §追記 2026-09-01)。
use super::{ArtifactAuditEventId, ArtifactAuditId, ArtifactWriteObservation};

mod saved;

pub use saved::ArtifactSaved;

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
