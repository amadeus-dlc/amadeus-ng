//! `ScopeSlugError` — `ScopeSlug::parse` が拒否する文法違反。

use std::fmt;

/// `ScopeSlug::parse` が拒否する文法違反。
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
