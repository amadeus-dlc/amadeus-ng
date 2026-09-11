//! ArtifactAudit投影をIDで読むポート。
use super::{ArtifactAuditView, ReadModelReadError};
/// 指定IDの投影済み行を単純に読む。
pub trait ArtifactAuditDao {
    /// ID検索。
    /// # Errors
    /// リードモデルを読めない場合。
    fn find(&self, id: &str) -> Result<Option<ArtifactAuditView>, ReadModelReadError>;
}
