//! 質問記録の失敗。
use super::port::RepositoryError;
use core_command_domain::orchestration::{CommandError, IntentExecutionId};

/// 更新経路の失敗を原因ごとに保持する。
#[derive(Debug)]
pub enum InteractionCommandError {
    /// 回答固有の拒否。
    Answer(core_command_domain::orchestration::AnswerError),
    /// 集約の読込み・保存失敗。
    Repository(RepositoryError<IntentExecutionId>),
    /// 集約による拒否。
    Command(CommandError),
}
impl core::fmt::Display for InteractionCommandError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Answer(error) => write!(f, "answer: {error}"),
            Self::Repository(error) => write!(f, "repository: {error}"),
            Self::Command(error) => write!(f, "command: {error}"),
        }
    }
}
impl std::error::Error for InteractionCommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Answer(error) => Some(error),
            Self::Repository(error) => Some(error),
            Self::Command(error) => Some(error),
        }
    }
}
