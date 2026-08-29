//! `RepositoryError` — `WorkflowExecutionRepository` の失敗 (entities.md)。

use std::fmt;
use std::io::ErrorKind;
use std::path::PathBuf;

use core_command_domain::orchestration::IntentId;

use super::corrupt_cause::CorruptCause;

/// `WorkflowExecutionRepository` の失敗 (材料のみ — 逐語文言はアダプタ層)。
///
/// 本ポートの面が語るのは「集約 1 つの再構成・永続化」だけである。下位のイベントストア
/// (本家 event-store-adapter-rs) の失敗を本型へ写すのは Gateway 実装の責務であり、
/// ユースケース層は本家のエラー型を知らない (ADR-010 — 依存が入るのはドメインと
/// アダプタだけ)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryError {
    /// この識別子の集約がストアに無い (契約上は呼出側の前提違反 — C3)。
    NotFound {
        /// 探した集約識別子。
        intent_id: IntentId,
    },
    /// 楽観 version の不一致 (BR1.3)。ユースケースは再水和して 1 回だけ再試行する。
    Conflict {
        /// 書込側が前提とした version。
        expected: usize,
        /// ストアに実在した version。
        actual: usize,
    },
    /// ストア I/O の失敗 (`ErrorKind` を保持 — 監査 C24)。
    Io {
        /// OS / ドライバ由来の分類。
        kind: ErrorKind,
        /// 対象パス (分からない場合は `None`)。
        path: Option<PathBuf>,
    },
    /// 復号不能・スナップショット欠落・不変条件違反 (部分データは返さない — BR1.2)。
    Corrupt {
        /// 対象の集約識別子。
        aggregate_id: IntentId,
        /// 該当行の `seq_nr` (行が特定できない場合は `None`)。
        seq_nr: Option<usize>,
        /// 原因の分類。
        cause: CorruptCause,
    },
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepositoryError::NotFound { intent_id } => write!(f, "not found: {intent_id}"),
            RepositoryError::Conflict { expected, actual } => {
                write!(f, "conflict: expected {expected}, actual {actual}")
            }
            RepositoryError::Io { kind, path } => write!(
                f,
                "io: {kind:?} at {}",
                path.as_ref()
                    .map_or_else(|| "-".to_string(), |p| p.display().to_string())
            ),
            RepositoryError::Corrupt {
                aggregate_id,
                seq_nr,
                cause,
            } => write!(
                f,
                "corrupt: aggregate {aggregate_id}, seq_nr {}, cause {cause}",
                seq_nr.map_or_else(|| "-".to_string(), |n| n.to_string())
            ),
        }
    }
}

impl std::error::Error for RepositoryError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::CorruptCause;
    use core_command_domain::orchestration::IntentId;
    use std::io::ErrorKind;
    use std::path::PathBuf;

    const RAW_ID: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

    fn intent() -> IntentId {
        IntentId::parse(RAW_ID).unwrap()
    }

    #[test]
    fn the_not_found_carries_the_intent_id() {
        let err = RepositoryError::NotFound {
            intent_id: intent(),
        };
        assert_eq!(err.to_string(), format!("not found: {RAW_ID}"));
    }

    #[test]
    fn the_conflict_carries_both_versions() {
        let err = RepositoryError::Conflict {
            expected: 3,
            actual: 5,
        };
        assert_eq!(err.to_string(), "conflict: expected 3, actual 5");
    }

    #[test]
    fn the_io_failure_carries_the_kind_and_the_path() {
        let err = RepositoryError::Io {
            kind: ErrorKind::PermissionDenied,
            path: Some(PathBuf::from("/tmp/store")),
        };
        assert_eq!(err.to_string(), "io: PermissionDenied at /tmp/store");
    }

    #[test]
    fn the_corrupt_failure_carries_the_aggregate_the_sequence_and_the_cause() {
        let err = RepositoryError::Corrupt {
            aggregate_id: intent(),
            seq_nr: Some(2),
            cause: CorruptCause::SequenceGap,
        };
        assert_eq!(
            err.to_string(),
            format!("corrupt: aggregate {RAW_ID}, seq_nr 2, cause sequence gap")
        );
    }

    #[test]
    fn the_missing_material_is_rendered_as_a_dash() {
        // 場所も `seq_nr` も分からない失敗はありうる (材料の欠落を空白で誤魔化さない)。
        assert_eq!(
            RepositoryError::Io {
                kind: ErrorKind::WouldBlock,
                path: None,
            }
            .to_string(),
            "io: WouldBlock at -"
        );
        assert_eq!(
            RepositoryError::Corrupt {
                aggregate_id: intent(),
                seq_nr: None,
                cause: CorruptCause::MissingSnapshot,
            }
            .to_string(),
            format!("corrupt: aggregate {RAW_ID}, seq_nr -, cause missing snapshot")
        );
    }

    #[test]
    fn the_error_is_a_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(RepositoryError::Conflict {
            expected: 0,
            actual: 1,
        });
        assert_eq!(err.to_string(), "conflict: expected 0, actual 1");
    }

    #[test]
    fn failures_compare_by_value() {
        assert_eq!(
            RepositoryError::NotFound {
                intent_id: intent()
            },
            RepositoryError::NotFound {
                intent_id: intent()
            }
        );
        assert_ne!(
            RepositoryError::NotFound {
                intent_id: intent()
            },
            RepositoryError::Conflict {
                expected: 0,
                actual: 0
            }
        );
    }
}
