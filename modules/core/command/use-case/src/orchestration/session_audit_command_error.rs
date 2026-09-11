//! セッション観測コマンドの失敗。
use super::RepositoryError;
use core_command_domain::workspace::{SessionAuditError, SessionAuditId};
/// ドメイン拒否と媒体失敗を区別する。
#[derive(Debug)]
pub enum SessionAuditCommandError {
    /// 観測の不変条件による拒否。
    Domain(SessionAuditError),
    /// 保存媒体の失敗。
    Repository(RepositoryError<SessionAuditId>),
}
impl std::fmt::Display for SessionAuditCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Domain(e) => e.fmt(f),
            Self::Repository(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for SessionAuditCommandError {}
impl From<SessionAuditError> for SessionAuditCommandError {
    fn from(e: SessionAuditError) -> Self {
        Self::Domain(e)
    }
}
impl From<RepositoryError<SessionAuditId>> for SessionAuditCommandError {
    fn from(e: RepositoryError<SessionAuditId>) -> Self {
        Self::Repository(e)
    }
}
