//! 学びの記録に失敗した原因。
use super::RepositoryError;
use core_command_domain::orchestration::{CommandError, IntentExecutionId};
/// 学びが 0 件だったこととは異なる、記録操作の失敗。
#[derive(Debug)]
pub enum LearningCaptureError {
    /// 集約の読込み・保存失敗。
    Repository(RepositoryError<IntentExecutionId>),
    /// 集約による拒否（計画に無いステージなど）。
    Command(CommandError),
}
impl core::fmt::Display for LearningCaptureError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Repository(error) => write!(f, "repository: {error}"),
            Self::Command(error) => write!(f, "command: {error}"),
        }
    }
}
impl std::error::Error for LearningCaptureError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Repository(error) => Some(error),
            Self::Command(error) => Some(error),
        }
    }
}
