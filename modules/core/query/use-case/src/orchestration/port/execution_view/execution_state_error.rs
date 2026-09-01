//! `ExecutionStateError` — [`ExecutionStateView`] の構築が拒否した形。
//!
//! リードモデルとして成立しない (= 判断の土台にできない) 観測だけを拒否する。運ぶのは
//! **材料だけ** (一致しなかった slug の逐語) で、利用者向けの文言は出す側が組む
//! (`coding-rules/error-handling.md`)。
//!
//! [`ExecutionStateView`]: super::ExecutionStateView

/// [`ExecutionStateView`] の構築が拒否する形。
///
/// リードモデルとして成立しない (= 判断の土台にできない) 観測だけを拒否する。文言は
/// 出す側が組む (`coding-rules/error-handling.md`)。
///
/// [`ExecutionStateView`]: super::ExecutionStateView
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStateError {
    /// Stage Progress の行が 1 本も無い (カーソルの置き場が無い)。
    NoStages,
    /// `Current Stage` が Stage Progress のどの行とも一致しない。
    UnknownCursor {
        /// 一致しなかった slug (逐語)。
        stage: String,
    },
    /// `Parked At Stage` が Stage Progress のどの行とも一致しない。
    UnknownParkedStage {
        /// 一致しなかった slug (逐語)。
        stage: String,
    },
}

impl std::fmt::Display for ExecutionStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionStateError::NoStages => f.write_str("no stage progress rows"),
            ExecutionStateError::UnknownCursor { stage } => {
                write!(f, "unknown current stage {stage:?}")
            }
            ExecutionStateError::UnknownParkedStage { stage } => {
                write!(f, "unknown parked stage {stage:?}")
            }
        }
    }
}

impl std::error::Error for ExecutionStateError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rejection_carries_material_not_wording() {
        assert_eq!(
            ExecutionStateError::NoStages.to_string(),
            "no stage progress rows"
        );
        assert_eq!(
            ExecutionStateError::UnknownCursor {
                stage: "ghost".to_string()
            }
            .to_string(),
            "unknown current stage \"ghost\""
        );
        let boxed: Box<dyn std::error::Error> = Box::new(ExecutionStateError::UnknownParkedStage {
            stage: "ghost".to_string(),
        });
        assert_eq!(boxed.to_string(), "unknown parked stage \"ghost\"");
    }
}
