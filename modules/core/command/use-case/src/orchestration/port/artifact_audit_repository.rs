//! ArtifactAudit集約の取得と保存ポート。
use super::RepositoryError;
use core_command_domain::workspace::{ArtifactAudit, ArtifactAuditEvent, ArtifactAuditId};
/// 成果物保存監査の媒体境界。
#[allow(
    async_fn_in_trait,
    reason = "既存Repositoryポートと同じcurrent_thread契約"
)]
pub trait ArtifactAuditRepository {
    /// 指定監査集約を再構成する。
    /// # Errors
    /// 不在・破損・I/Oの失敗。
    async fn find_by_id(
        &self,
        id: &ArtifactAuditId,
    ) -> Result<ArtifactAudit, RepositoryError<ArtifactAuditId>>;
    /// 集約が生成した1件の保存観測を永続化する。
    /// # Errors
    /// 競合・破損・I/Oの失敗。
    async fn store(
        &mut self,
        event: &ArtifactAuditEvent,
        aggregate: &ArtifactAudit,
    ) -> Result<(), RepositoryError<ArtifactAuditId>>;
}
