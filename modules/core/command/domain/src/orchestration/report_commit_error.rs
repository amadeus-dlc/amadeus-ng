//! report 判断の適用を拒む材料。
use super::command_error::CommandError;
use super::transition_step::TransitionStep;
use std::fmt;

/// 判断された遷移をコミットできない理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportCommitError {
    /// 対象コマンドのガードが拒否した。
    Transition {
        /// 拒否された遷移。
        step: TransitionStep,
        /// 集約の拒否理由。
        error: CommandError,
    },
    /// 対応する集約コマンドが存在しない。
    Unwired {
        /// 未対応の遷移。
        step: TransitionStep,
    },
}
impl fmt::Display for ReportCommitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transition { step, error } => write!(f, "{step:?}: {error}"),
            Self::Unwired { step } => write!(f, "unwired transition: {step:?}"),
        }
    }
}
impl std::error::Error for ReportCommitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transition { error, .. } => Some(error),
            Self::Unwired { .. } => None,
        }
    }
}
