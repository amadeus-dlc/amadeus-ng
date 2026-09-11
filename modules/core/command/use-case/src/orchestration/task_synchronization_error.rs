//! TaskUpdate同期の拒否と保存失敗。
use super::RepositoryError;
use core_command_domain::orchestration::{CommandError, IntentExecutionId};
/// 同期が保存できなかった原因。
#[derive(Debug)]
pub enum TaskSynchronizationError {
    /// stageやイベント通番の拒否。
    Command(CommandError),
    /// 実行の読取り・保存失敗。
    Repository(RepositoryError<IntentExecutionId>),
}
impl core::fmt::Display for TaskSynchronizationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Command(e) => write!(f, "command: {e}"),
            Self::Repository(e) => write!(f, "repository: {e}"),
        }
    }
}
impl std::error::Error for TaskSynchronizationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Command(e) => Some(e),
            Self::Repository(e) => Some(e),
        }
    }
}
