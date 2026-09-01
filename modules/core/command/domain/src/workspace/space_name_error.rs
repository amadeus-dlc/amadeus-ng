//! `SpaceNameError` — `SpaceName::parse` の拒否理由。

/// `parse` の拒否理由。正規化 (小文字化・区切り置換) は一切しない — 受理か拒否のみ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpaceNameError {
    /// 空文字列。
    Empty,
    /// 先頭は `[a-z]` 必須。
    InvalidLeading(char),
    /// 2 文字目以降は `[a-z0-9-]` のみ。
    InvalidChar(char),
}
