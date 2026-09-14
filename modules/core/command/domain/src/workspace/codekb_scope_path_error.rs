//! `CodekbScopePathError` — [`CodekbScopePath::parse`](super::CodekbScopePath::parse) の拒否理由。

/// 走査範囲のパスとして受理できなかった理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodekbScopePathError {
    /// 空、または空白だけの綴り。
    Empty,
}

impl std::fmt::Display for CodekbScopePathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("empty codekb scope path")
    }
}

impl std::error::Error for CodekbScopePathError {}
