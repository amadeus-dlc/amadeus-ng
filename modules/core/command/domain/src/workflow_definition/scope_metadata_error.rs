//! `ScopeMetadataError` — `ScopeMetadata` の構成が拒否される理由。

/// `ScopeMetadata` の構成が拒否される理由。frontmatter の必須キー欠落のみ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeMetadataError {
    /// `name:` が無い / 空 (upstream: `missing required frontmatter: name`)。
    MissingName,
}
