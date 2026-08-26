//! `ApplyError` — `apply_event` がイベントを適用できない理由 (functional-spec §5)。

use std::fmt;

use crate::workflow_definition::StageSlug;

/// 再水和・リプレイの失敗材料 (U3 は `Corrupt` に写す)。適用前の状態は保たれる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyError {
    /// 封筒の `seq_nr` が現在値 + 1 でない (BR2.1)。
    SequenceGap {
        /// 集約が期待した `seq_nr` (現在値 + 1)。
        expected: usize,
        /// イベント封筒が持っていた `seq_nr`。
        actual: usize,
    },
    /// ペイロードのステージ slug が `stages` に無い。
    UnknownStage(StageSlug),
    /// 適用後に集約不変条件が破れた (材料は不変条件名)。
    InvariantViolation(String),
}

impl fmt::Display for ApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApplyError::SequenceGap { expected, actual } => {
                write!(f, "sequence gap: expected {expected}, actual {actual}")
            }
            ApplyError::UnknownStage(slug) => write!(f, "unknown stage: {slug}"),
            ApplyError::InvariantViolation(reason) => {
                write!(f, "invariant violation: {reason}")
            }
        }
    }
}

impl std::error::Error for ApplyError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_definition::StageSlug;

    #[test]
    fn the_sequence_gap_carries_both_numbers() {
        let err = ApplyError::SequenceGap {
            expected: 4,
            actual: 7,
        };
        assert_eq!(err.to_string(), "sequence gap: expected 4, actual 7");
    }

    #[test]
    fn the_unknown_stage_carries_the_slug() {
        let err = ApplyError::UnknownStage(StageSlug::parse("no-such-stage").unwrap());
        assert_eq!(err.to_string(), "unknown stage: no-such-stage");
    }

    #[test]
    fn the_invariant_violation_carries_the_reason() {
        let err = ApplyError::InvariantViolation("cursor_in_scope".to_string());
        assert_eq!(err.to_string(), "invariant violation: cursor_in_scope");
    }

    #[test]
    fn the_error_is_a_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(ApplyError::SequenceGap {
            expected: 1,
            actual: 2,
        });
        assert_eq!(err.to_string(), "sequence gap: expected 1, actual 2");
    }

    #[test]
    fn rejections_compare_by_value() {
        assert_eq!(
            ApplyError::InvariantViolation("a".to_string()),
            ApplyError::InvariantViolation("a".to_string())
        );
        assert_ne!(
            ApplyError::InvariantViolation("a".to_string()),
            ApplyError::InvariantViolation("b".to_string())
        );
    }
}
