//! 自己診断の実施・保存に失敗した原因。
use super::RepositoryError;
use core_command_domain::workspace::{WorkspaceDoctorError, WorkspaceDoctorId};
/// 集約の拒否と保存失敗を区別する。
#[derive(Debug)]
pub enum WorkspaceDoctorCommandError {
    /// ドメインの履歴条件違反。
    Domain(WorkspaceDoctorError),
    /// 保存・再構成の失敗。
    Repository(RepositoryError<WorkspaceDoctorId>),
}
impl std::fmt::Display for WorkspaceDoctorCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Domain(error) => error.fmt(f),
            Self::Repository(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for WorkspaceDoctorCommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Domain(error) => Some(error),
            Self::Repository(error) => Some(error),
        }
    }
}
