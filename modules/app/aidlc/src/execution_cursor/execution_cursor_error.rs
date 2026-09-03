//! `ExecutionCursorError` — 実行カーソルの読み書きの失敗（材料のみ）。

use std::error::Error;
use std::fmt;
use std::io::ErrorKind;
use std::path::PathBuf;

/// 実行カーソルの読み書きの失敗（材料のみ — 利用者向けの文言は出す側が組む。
/// `coding-rules/error-handling.md`）。
///
/// **不在はここに無い。** カーソルがまだ据わっていないのは fresh なワークスペースの正常な
/// 姿であり失敗ではないので、[`super::ExecutionCursor::read`] は `Ok(None)` で答える。
/// 本型が表すのは「在るのに読めない」「在るが読めても意味を成さない」の 2 つだけである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionCursorError {
    /// 読めない・書けない（権限・親ディレクトリの不在など）。
    Io {
        /// OS 由来の分類。
        kind: ErrorKind,
        /// 対象パス。
        path: PathBuf,
    },
    /// 在るが 2 つの識別子として読めない（行数違い・識別子の文法外）。
    Malformed {
        /// 対象パス。
        path: PathBuf,
    },
}

impl fmt::Display for ExecutionCursorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // 材料の並びは `RepositoryError::Io` と揃える（家風 — `io: <分類> at <パス>`）。
            ExecutionCursorError::Io { kind, path } => {
                write!(f, "io: {kind:?} at {}", path.display())
            }
            ExecutionCursorError::Malformed { path } => {
                write!(f, "malformed execution cursor at {}", path.display())
            }
        }
    }
}

impl Error for ExecutionCursorError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_io_variant_names_the_classification_and_the_path() {
        let error = ExecutionCursorError::Io {
            kind: ErrorKind::PermissionDenied,
            path: PathBuf::from("/w/record/.aidlc-execution"),
        };

        assert_eq!(
            error.to_string(),
            "io: PermissionDenied at /w/record/.aidlc-execution"
        );
    }

    #[test]
    fn the_malformed_variant_names_the_path() {
        let error = ExecutionCursorError::Malformed {
            path: PathBuf::from("/w/record/.aidlc-execution"),
        };

        assert_eq!(
            error.to_string(),
            "malformed execution cursor at /w/record/.aidlc-execution"
        );
    }

    /// 材料しか運ばないので原因連鎖は持たない（連鎖を辿る側が空振りしないことの固定）。
    #[test]
    fn the_error_carries_no_cause_chain() {
        let error = ExecutionCursorError::Malformed {
            path: PathBuf::from("/w/record/.aidlc-execution"),
        };

        assert!(error.source().is_none());
    }
}
