//! `StateError` — `from_state` が状態の写し (memento) を受け入れられない理由。

use std::fmt;

/// 状態の写し (memento) が集約不変条件を満たさない (U3 は `Corrupt` に写す — C3)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateError {
    /// 破れた不変条件の材料 (文言はアダプタ層)。
    InvariantViolation(String),
}

impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StateError::InvariantViolation(reason) => {
                write!(f, "invariant violation: {reason}")
            }
        }
    }
}

impl std::error::Error for StateError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_invariant_violation_carries_the_reason() {
        let err = StateError::InvariantViolation("length mismatch: checkbox".to_string());
        assert_eq!(
            err.to_string(),
            "invariant violation: length mismatch: checkbox"
        );
    }

    #[test]
    fn the_error_is_a_std_error() {
        let err: Box<dyn std::error::Error> =
            Box::new(StateError::InvariantViolation("seq_nr".to_string()));
        assert_eq!(err.to_string(), "invariant violation: seq_nr");
    }

    #[test]
    fn rejections_compare_by_value() {
        assert_eq!(
            StateError::InvariantViolation("a".to_string()),
            StateError::InvariantViolation("a".to_string())
        );
        assert_ne!(
            StateError::InvariantViolation("a".to_string()),
            StateError::InvariantViolation("b".to_string())
        );
    }
}
