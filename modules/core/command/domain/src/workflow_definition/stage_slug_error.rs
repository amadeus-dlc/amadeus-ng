//! `StageSlugError` — `StageSlug::parse` が拒否する文法違反。

use std::fmt;

/// `StageSlug::parse` が拒否する文法違反。
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
