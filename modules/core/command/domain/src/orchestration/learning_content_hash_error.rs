//! `LearningContentHashError` — 学びの同一性の綴り違反。

/// 64 桁の小文字 16 進として読めなかった綴り。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearningContentHashError {
    given: String,
}

impl LearningContentHashError {
    /// 拒否した綴りを運ぶ。
    #[must_use]
    pub fn new(given: &str) -> LearningContentHashError {
        LearningContentHashError {
            given: given.to_string(),
        }
    }

    /// 拒否した綴り。
    #[must_use]
    pub fn given(&self) -> &str {
        &self.given
    }
}

impl core::fmt::Display for LearningContentHashError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "content hash must be 64 lowercase hex digits: {}",
            self.given
        )
    }
}

impl std::error::Error for LearningContentHashError {}

#[cfg(test)]
mod tests {
    use super::LearningContentHashError;

    #[test]
    fn the_error_carries_the_rejected_spelling() {
        let error = LearningContentHashError::new("abc");
        assert_eq!(error.given(), "abc");
        assert_eq!(
            error.to_string(),
            "content hash must be 64 lowercase hex digits: abc"
        );
    }
}
