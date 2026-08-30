//! `IntentExecutionError` — 集約の完全コンストラクタが拒む材料。
//!
//! [`IntentExecution::new`](super::IntentExecution::new) の失敗面 ([`Intent`] の
//! [`IntentError`](super::IntentError) と対)。ジャーナルの再生 (`apply_event`) は壊れた歴史を
//! クラッシュで止めるが、完全コンストラクタは**ストア境界の読取** (復元 DTO) からも呼ばれる
//! ので、壊れた行はアダプタが `Corrupt` へ写せるよう `Err` で返す (BR1.5)。
//!
//! [`Intent`]: super::Intent

use std::fmt;

/// 構築材料が集約の不変条件を満たさない (材料は理由の一文)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentExecutionError {
    reason: String,
}

impl IntentExecutionError {
    /// 拒否の理由を束ねる。
    #[must_use]
    pub fn new(reason: impl Into<String>) -> IntentExecutionError {
        IntentExecutionError {
            reason: reason.into(),
        }
    }

    /// 拒否の理由 (診断表示の材料)。
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl fmt::Display for IntentExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid intent execution: {}", self.reason)
    }
}

impl std::error::Error for IntentExecutionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_error_renders_its_reason() {
        let error = IntentExecutionError::new("cursor out of bounds");
        assert_eq!(error.reason(), "cursor out of bounds");
        assert_eq!(
            error.to_string(),
            "invalid intent execution: cursor out of bounds"
        );
    }
}
