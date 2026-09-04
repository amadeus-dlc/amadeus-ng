//! `StageSlugError` — [`StageSlugView::parse`] が拒否した文法違反。
//!
//! 運ぶのは**材料だけ** (どの文字が違反したか) で、利用者向けの逐語文言は出す側が組む
//! (`coding-rules/error-handling.md`)。
//!
//! [`StageSlugView::parse`]: super::StageSlugView::parse

use std::fmt;

/// `StageSlugView::parse` が拒否する文法違反。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageSlugError {
    /// 入力が空文字列。
    Empty,
    /// 先頭は `[a-z]` 必須。
    InvalidLeading(char),
    /// 2 文字目以降は `[a-z0-9-]` のみ。
    InvalidChar(char),
}

impl fmt::Display for StageSlugError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StageSlugError::Empty => f.write_str("empty"),
            StageSlugError::InvalidLeading(c) => write!(f, "leading character '{c}'"),
            StageSlugError::InvalidChar(c) => write!(f, "invalid character '{c}'"),
        }
    }
}

impl std::error::Error for StageSlugError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rejection_carries_material_not_wording() {
        assert_eq!(StageSlugError::Empty.to_string(), "empty");
        assert_eq!(
            StageSlugError::InvalidLeading('1').to_string(),
            "leading character '1'"
        );
        assert_eq!(
            StageSlugError::InvalidChar('_').to_string(),
            "invalid character '_'"
        );
    }
}
