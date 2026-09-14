//! `SnapshotCodekbError` — `SnapshotCodekbUseCase` の失敗。

use std::fmt;

use core_command_domain::workspace::CodekbRepoId;

use super::port::RepositoryError;

/// [`super::SnapshotCodekbUseCase`] の失敗（材料のみ — 逐語文言は出す側が組む）。
// `Clone` / `PartialEq` は実装しない — `Corrupt` の `source` (原因連鎖) が比較・複製不能で
// あるためである（他のユースケース封筒と同じ理由）。
#[derive(Debug)]
pub enum SnapshotCodekbError {
    /// ストアの再構成の失敗（ポートからそのまま伝播）。
    Repository(RepositoryError<CodekbRepoId>),
}

impl fmt::Display for SnapshotCodekbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SnapshotCodekbError::Repository(error) => write!(f, "repository: {error}"),
        }
    }
}

impl std::error::Error for SnapshotCodekbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SnapshotCodekbError::Repository(error) => Some(error),
        }
    }
}

impl From<RepositoryError<CodekbRepoId>> for SnapshotCodekbError {
    fn from(error: RepositoryError<CodekbRepoId>) -> SnapshotCodekbError {
        SnapshotCodekbError::Repository(error)
    }
}
