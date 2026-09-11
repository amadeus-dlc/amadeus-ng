//! `LearningCandidateIdError` — 候補番号の綴り違反。

/// 監査行の 1 ラベル 1 行を壊す綴り。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearningCandidateIdError {
    given: String,
}

impl LearningCandidateIdError {
    /// 拒否した綴りを運ぶ。
    #[must_use]
    pub fn new(given: &str) -> LearningCandidateIdError {
        LearningCandidateIdError {
            given: given.to_string(),
        }
    }

    /// 拒否した綴り。
    #[must_use]
    pub fn given(&self) -> &str {
        &self.given
    }
}

impl core::fmt::Display for LearningCandidateIdError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "candidate id must be one non-empty label on one line: {:?}",
            self.given
        )
    }
}

impl std::error::Error for LearningCandidateIdError {}

#[cfg(test)]
mod tests {
    use super::LearningCandidateIdError;

    #[test]
    fn the_error_carries_the_rejected_spelling() {
        let error = LearningCandidateIdError::new("c1\n");
        assert_eq!(error.given(), "c1\n");
        assert!(error.to_string().contains("one non-empty label"));
    }
}
