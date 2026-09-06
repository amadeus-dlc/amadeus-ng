//! `TransitionStepsError` — 遷移サブコマンド列の不変条件を破った形。

use std::fmt;

use super::transition_step::TransitionStep;

/// [`TransitionSteps`] が満たすべき不変条件の違反 (材料のみ — 利用者向け文言はアダプタ層)。
///
/// [`TransitionSteps`]: super::TransitionSteps
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionStepsError {
    /// 同じ段が 2 回以上現れる (1 回の報告適用で同じ遷移を 2 度踏むことはない)。
    Duplicate {
        /// 列の順で最初に重複した段。
        step: TransitionStep,
    },
}

impl fmt::Display for TransitionStepsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // 材料は変種名で書く — `GateStartRecovered` と `GateStart` は
            // `subcommand()` の綴りが同じ `gate-start` で、綴りでは区別が付かない。
            TransitionStepsError::Duplicate { step } => {
                write!(f, "duplicate transition step: {step:?}")
            }
        }
    }
}

impl std::error::Error for TransitionStepsError {}

#[cfg(test)]
mod tests {
    use super::TransitionStepsError;
    use crate::orchestration::TransitionStep;

    #[test]
    fn the_violation_renders_its_material() {
        assert_eq!(
            TransitionStepsError::Duplicate {
                step: TransitionStep::Approve,
            }
            .to_string(),
            "duplicate transition step: Approve"
        );
    }

    #[test]
    fn the_two_gate_start_spellings_are_told_apart_in_the_material() {
        assert_eq!(
            TransitionStepsError::Duplicate {
                step: TransitionStep::GateStartRecovered,
            }
            .to_string(),
            "duplicate transition step: GateStartRecovered",
            "subcommand の綴りは両者とも gate-start なので材料は変種名で書く"
        );
    }

    #[test]
    fn the_violation_is_a_std_error() {
        let error: &dyn std::error::Error = &TransitionStepsError::Duplicate {
            step: TransitionStep::Skip,
        };
        assert!(error.source().is_none(), "材料を自分で持つので連鎖しない");
    }

    #[test]
    fn violations_compare_by_value() {
        assert_eq!(
            TransitionStepsError::Duplicate {
                step: TransitionStep::Approve
            },
            TransitionStepsError::Duplicate {
                step: TransitionStep::Approve
            }
        );
        assert_ne!(
            TransitionStepsError::Duplicate {
                step: TransitionStep::Approve
            },
            TransitionStepsError::Duplicate {
                step: TransitionStep::Reject
            }
        );
    }
}
