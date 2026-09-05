//! 名指しの隔離実行の拒否材料。
use super::command_error::CommandError;
use std::fmt;

/// 名指しされたステージの隔離実行を記録できない理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamedStageRunError {
    /// 計画に対象ステージが存在しない。
    UnknownStage,
    /// 集約コマンドが拒否した。
    Command(CommandError),
}
impl fmt::Display for NamedStageRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownStage => f.write_str("unknown stage"),
            Self::Command(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for NamedStageRunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::UnknownStage => None,
            Self::Command(error) => Some(error),
        }
    }
}
