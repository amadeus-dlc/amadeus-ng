//! 停止制御コマンドの拒否。
use super::RepositoryError;
use core_command_domain::orchestration::{ContinuationError, WorkflowContinuationId};
#[derive(Debug)]
/// 停止制御コマンドの拒否。
pub enum ContinuationCommandError {
    /// 入力または回数の拒否。
    Domain(ContinuationError),
    /// 保存境界の失敗。
    Repository(RepositoryError<WorkflowContinuationId>),
    /// 参照する実行の取得失敗。
    ExecutionRepository(RepositoryError<core_command_domain::orchestration::IntentExecutionId>),
}
impl From<ContinuationError> for ContinuationCommandError {
    fn from(error: ContinuationError) -> Self {
        Self::Domain(error)
    }
}
impl From<RepositoryError<WorkflowContinuationId>> for ContinuationCommandError {
    fn from(error: RepositoryError<WorkflowContinuationId>) -> Self {
        Self::Repository(error)
    }
}
impl std::fmt::Display for ContinuationCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Domain(error) => error.fmt(f),
            Self::Repository(error) => error.fmt(f),
            Self::ExecutionRepository(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for ContinuationCommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Domain(error) => Some(error),
            Self::Repository(error) => Some(error),
            Self::ExecutionRepository(error) => Some(error),
        }
    }
}
