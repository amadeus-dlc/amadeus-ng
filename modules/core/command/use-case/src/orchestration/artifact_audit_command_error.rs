//! 成果物保存観測コマンドの失敗。
use super::RepositoryError;
use core_command_domain::workspace::{ArtifactAuditId, HookHealthError};
/// ドメイン拒否と媒体失敗を区別する。
#[derive(Debug)]
pub enum ArtifactAuditCommandError {
    /// 観測の不変条件による拒否。
    Domain(HookHealthError),
    /// 保存媒体の失敗。
    Repository(RepositoryError<ArtifactAuditId>),
}
impl std::fmt::Display for ArtifactAuditCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Domain(e) => e.fmt(f),
            Self::Repository(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for ArtifactAuditCommandError {}
impl From<HookHealthError> for ArtifactAuditCommandError {
    fn from(e: HookHealthError) -> Self {
        Self::Domain(e)
    }
}
impl From<RepositoryError<ArtifactAuditId>> for ArtifactAuditCommandError {
    fn from(e: RepositoryError<ArtifactAuditId>) -> Self {
        Self::Repository(e)
    }
}
