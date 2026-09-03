//! `ScopeSlugError` — [`ScopeSlugView::parse`] が拒否した文法違反。
//!
//! 運ぶのは**材料だけ** (どの文字が違反したか) で、利用者向けの逐語文言は出す側が組む
//! (`coding-rules/error-handling.md`)。
//!
//! [`ScopeSlugView::parse`]: super::ScopeSlugView::parse

use std::fmt;

/// `ScopeSlugView::parse` が拒否する文法違反。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeSlugError {
    /// 入力が空文字列。
    Empty,
    /// 先頭は `[a-z]` 必須。
    InvalidLeading(char),
    /// 2 文字目以降は `[a-z0-9-]` のみ。
    InvalidChar(char),
}

impl fmt::Display for ScopeSlugError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScopeSlugError::Empty => f.write_str("empty"),
            ScopeSlugError::InvalidLeading(c) => write!(f, "leading character '{c}'"),
            ScopeSlugError::InvalidChar(c) => write!(f, "invalid character '{c}'"),
        }
    }
}

impl std::error::Error for ScopeSlugError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rejection_carries_material_not_wording() {
        assert_eq!(ScopeSlugError::Empty.to_string(), "empty");
        assert_eq!(
            ScopeSlugError::InvalidLeading('C').to_string(),
            "leading character 'C'"
        );
        assert_eq!(
            ScopeSlugError::InvalidChar('_').to_string(),
            "invalid character '_'"
        );
        let boxed: Box<dyn std::error::Error> = Box::new(ScopeSlugError::Empty);
        assert_eq!(boxed.to_string(), "empty");
    }
}
