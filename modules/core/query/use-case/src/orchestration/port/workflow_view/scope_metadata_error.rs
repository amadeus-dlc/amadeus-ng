//! `ScopeMetadataError` — [`ScopeMetadataView::new`] の構成が拒否される理由。
//!
//! frontmatter の必須キー欠落だけを拒否する。運ぶのは**材料だけ**で、利用者向けの逐語文言は
//! 出す側が組む (`coding-rules/error-handling.md`)。
//!
//! [`ScopeMetadataView::new`]: super::ScopeMetadataView::new

use std::fmt;

/// `ScopeMetadataView` の構成が拒否される理由。frontmatter の必須キー欠落のみ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeMetadataError {
    /// `name:` が無い / 空 (upstream: `missing required frontmatter: name`)。
    MissingName,
}

impl fmt::Display for ScopeMetadataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScopeMetadataError::MissingName => f.write_str("missing required frontmatter: name"),
        }
    }
}

impl std::error::Error for ScopeMetadataError {}
