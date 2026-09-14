//! `CodekbRepoIdError` — [`CodekbRepoId::parse`](super::CodekbRepoId::parse) の拒否理由。

/// リポジトリ識別子として受理できなかった理由。正規化 (小文字化・区切り置換) は一切しない —
/// 受理か拒否のみである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodekbRepoIdError {
    /// 空文字列。
    Empty,
    /// 先頭は英数字 (`[A-Za-z0-9]`) 必須。
    InvalidLeading(char),
    /// 2 文字目以降は `[A-Za-z0-9._-]` のみ。
    InvalidChar(char),
}

impl std::fmt::Display for CodekbRepoIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodekbRepoIdError::Empty => f.write_str("empty repo name"),
            CodekbRepoIdError::InvalidLeading(c) => {
                write!(f, "repo name must start with an alphanumeric, got {c:?}")
            }
            CodekbRepoIdError::InvalidChar(c) => {
                write!(f, "repo name must not contain {c:?}")
            }
        }
    }
}

impl std::error::Error for CodekbRepoIdError {}
