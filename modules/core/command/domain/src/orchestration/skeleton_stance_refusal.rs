//! skeleton stance の記録を拒んだ判断と対象文脈。
use super::command_error::CommandError;
use crate::workflow_definition::StageSlug;
use std::fmt;

/// stance の記録を拒んだ理由と、その判断に使用した計画の文脈。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkeletonStanceRefusal {
    /// 集約のガードが拒否した。
    Rejected {
        /// 判断時のカーソルが指すステージ。
        stage: Option<StageSlug>,
        /// 判断に使用した計画のスコープ。
        scope: String,
        /// 集約の拒否理由。
        error: CommandError,
    },
}
impl fmt::Display for SkeletonStanceRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self::Rejected {
            stage,
            scope,
            error,
        } = self;
        write!(f, "{stage:?} ({scope}): {error}")
    }
}
impl std::error::Error for SkeletonStanceRefusal {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        let Self::Rejected { error, .. } = self;
        Some(error)
    }
}
