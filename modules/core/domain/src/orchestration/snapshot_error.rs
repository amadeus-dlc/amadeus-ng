//! `SnapshotError` — `from_snapshot` がスナップショットを受け入れられない理由。

use std::fmt;

/// スナップショットが集約不変条件を満たさない (U3 は `Corrupt` に写す — C3)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    /// 破れた不変条件の材料 (文言はアダプタ層)。
    InvariantViolation(String),
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SnapshotError::InvariantViolation(reason) => {
                write!(f, "invariant violation: {reason}")
            }
        }
    }
}

impl std::error::Error for SnapshotError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_invariant_violation_carries_the_reason() {
        let err = SnapshotError::InvariantViolation("length mismatch: checkbox".to_string());
        assert_eq!(
            err.to_string(),
            "invariant violation: length mismatch: checkbox"
        );
    }

    #[test]
    fn the_error_is_a_std_error() {
        let err: Box<dyn std::error::Error> =
            Box::new(SnapshotError::InvariantViolation("seq_nr".to_string()));
        assert_eq!(err.to_string(), "invariant violation: seq_nr");
    }

    #[test]
    fn rejections_compare_by_value() {
        assert_eq!(
            SnapshotError::InvariantViolation("a".to_string()),
            SnapshotError::InvariantViolation("a".to_string())
        );
        assert_ne!(
            SnapshotError::InvariantViolation("a".to_string()),
            SnapshotError::InvariantViolation("b".to_string())
        );
    }
}
