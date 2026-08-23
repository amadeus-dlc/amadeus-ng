//! `EventStoreError` / `CorruptCause` — `EventStore` と `JournalReader` の失敗 (entities.md)。

use std::fmt;
use std::io::ErrorKind;
use std::path::PathBuf;

use super::global_seq_nr::GlobalSeqNr;
use super::projection_name::ProjectionName;

/// `EventStore` / `JournalReader` の失敗 (材料のみ — 利用者向けの逐語文言はアダプタ層の
/// message-catalog が組み立てる。`coding-rules/error-handling.md`)。
///
/// Repository 面へは [`RepositoryError::from_event_store`] が写す。
///
/// [`RepositoryError::from_event_store`]: super::repository_error::RepositoryError::from_event_store
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventStoreError {
    /// 楽観 version の不一致、またはジャーナルの `UNIQUE(aggregate_id, seq_nr)` 違反 (BR1.3)。
    Conflict {
        /// 書込側が前提とした version (= 永続化済みの最後の `seq_nr`)。
        expected: u64,
        /// ストアに実在した version。
        actual: u64,
    },
    /// ストア I/O の失敗。`ErrorKind` を保持する (監査 C24)。busy_timeout 超過は
    /// `ErrorKind::WouldBlock` に写す (NFR3.5)。
    Io {
        /// OS / ドライバ由来の分類。
        kind: ErrorKind,
        /// 対象パス (分からない場合は `None`)。
        path: Option<PathBuf>,
    },
    /// 行を読めたがドメインへ写せない (復号不能・スナップショット欠落・不変条件違反)。
    ///
    /// `aggregate_id` は**行に入っていた生文字列**。ドメイン型ではないのは、破損した行の
    /// 識別子が `IntentId` として妥当とは限らないためである。
    Corrupt {
        /// 行が名乗っていた集約識別子 (生文字列)。
        aggregate_id: String,
        /// 該当行の `seq_nr` (行が特定できない場合は `None`)。
        seq_nr: Option<u64>,
        /// 原因の分類。
        cause: CorruptCause,
    },
    /// `PRAGMA user_version` が対応範囲外 (BR2.1)。
    Schema {
        /// ストアに書かれていた版。
        found: u32,
        /// 本実装が対応する版。
        supported: u32,
    },
    /// チェックポイントを現在値より後ろへ動かそうとした (BR1.4)。
    CheckpointRegression {
        /// 対象の投影。
        projection: ProjectionName,
        /// 現在のチェックポイント。
        current: GlobalSeqNr,
        /// 要求された位置。
        requested: GlobalSeqNr,
    },
}

/// `Corrupt` の原因分類 (材料)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CorruptCause {
    /// ジャーナル行はあるのにスナップショット行が無い (BR1.2)。
    MissingSnapshot,
    /// ペイロードの JSON がワイヤ形式に写せない (未知フィールド・型不一致・値の形式違反)。
    UndecodablePayload,
    /// `type` タグが 12 語の閉集合の外。
    UnknownEventType,
    /// 行の `schema_version` が対応版と違う。
    SchemaVersion,
    /// 復元・適用の結果が集約不変条件を破る (`from_state` / `apply_event` の `Err`)。
    InvariantViolation,
    /// 集約内の `seq_nr` が連続していない (呼出側の不整合、またはジャーナルの欠損)。
    SequenceGap,
}

/// `Option<PathBuf>` / `Option<u64>` を材料として描く (欠落は `-`)。
fn render_path(path: Option<&PathBuf>) -> String {
    path.map_or_else(|| "-".to_string(), |p| p.display().to_string())
}

/// 欠落しうる `seq_nr` を材料として描く。
fn render_seq_nr(seq_nr: Option<u64>) -> String {
    seq_nr.map_or_else(|| "-".to_string(), |n| n.to_string())
}

impl fmt::Display for EventStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventStoreError::Conflict { expected, actual } => {
                write!(f, "conflict: expected {expected}, actual {actual}")
            }
            EventStoreError::Io { kind, path } => {
                write!(f, "io: {kind:?} at {}", render_path(path.as_ref()))
            }
            EventStoreError::Corrupt {
                aggregate_id,
                seq_nr,
                cause,
            } => write!(
                f,
                "corrupt: aggregate {aggregate_id}, seq_nr {}, cause {cause}",
                render_seq_nr(*seq_nr)
            ),
            EventStoreError::Schema { found, supported } => {
                write!(f, "schema: found {found}, supported {supported}")
            }
            EventStoreError::CheckpointRegression {
                projection,
                current,
                requested,
            } => write!(
                f,
                "checkpoint regression: projection {projection}, current {current}, requested {requested}"
            ),
        }
    }
}

impl std::error::Error for EventStoreError {}

impl fmt::Display for CorruptCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CorruptCause::MissingSnapshot => "missing snapshot",
            CorruptCause::UndecodablePayload => "undecodable payload",
            CorruptCause::UnknownEventType => "unknown event type",
            CorruptCause::SchemaVersion => "schema version",
            CorruptCause::InvariantViolation => "invariant violation",
            CorruptCause::SequenceGap => "sequence gap",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::{GlobalSeqNr, ProjectionName};
    use std::io::ErrorKind;
    use std::path::PathBuf;

    fn projection() -> ProjectionName {
        ProjectionName::parse("state-file").unwrap()
    }

    #[test]
    fn the_conflict_carries_both_versions() {
        let err = EventStoreError::Conflict {
            expected: 3,
            actual: 4,
        };
        assert_eq!(err.to_string(), "conflict: expected 3, actual 4");
    }

    #[test]
    fn the_io_failure_carries_the_kind_and_the_path() {
        let err = EventStoreError::Io {
            kind: ErrorKind::NotFound,
            path: Some(PathBuf::from("/tmp/.aidlc-store.sqlite")),
        };
        assert_eq!(err.to_string(), "io: NotFound at /tmp/.aidlc-store.sqlite");
        let without_path = EventStoreError::Io {
            kind: ErrorKind::WouldBlock,
            path: None,
        };
        assert_eq!(without_path.to_string(), "io: WouldBlock at -");
    }

    #[test]
    fn the_corrupt_failure_carries_the_aggregate_the_sequence_and_the_cause() {
        let err = EventStoreError::Corrupt {
            aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
            seq_nr: Some(4),
            cause: CorruptCause::UndecodablePayload,
        };
        assert_eq!(
            err.to_string(),
            "corrupt: aggregate 01a02785-1bd8-76eb-aeea-5aa303ebd5b6, seq_nr 4, cause undecodable payload"
        );
        let without_seq = EventStoreError::Corrupt {
            aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
            seq_nr: None,
            cause: CorruptCause::MissingSnapshot,
        };
        assert_eq!(
            without_seq.to_string(),
            "corrupt: aggregate 01a02785-1bd8-76eb-aeea-5aa303ebd5b6, seq_nr -, cause missing snapshot"
        );
    }

    #[test]
    fn the_schema_failure_carries_the_found_and_supported_versions() {
        let err = EventStoreError::Schema {
            found: 2,
            supported: 1,
        };
        assert_eq!(err.to_string(), "schema: found 2, supported 1");
    }

    #[test]
    fn the_checkpoint_regression_carries_the_projection_and_both_positions() {
        let err = EventStoreError::CheckpointRegression {
            projection: projection(),
            current: GlobalSeqNr::new(9),
            requested: GlobalSeqNr::new(4),
        };
        assert_eq!(
            err.to_string(),
            "checkpoint regression: projection state-file, current 9, requested 4"
        );
    }

    #[test]
    fn every_corrupt_cause_renders_its_material() {
        assert_eq!(
            CorruptCause::MissingSnapshot.to_string(),
            "missing snapshot"
        );
        assert_eq!(
            CorruptCause::UndecodablePayload.to_string(),
            "undecodable payload"
        );
        assert_eq!(
            CorruptCause::UnknownEventType.to_string(),
            "unknown event type"
        );
        assert_eq!(CorruptCause::SchemaVersion.to_string(), "schema version");
        assert_eq!(
            CorruptCause::InvariantViolation.to_string(),
            "invariant violation"
        );
        assert_eq!(CorruptCause::SequenceGap.to_string(), "sequence gap");
    }

    #[test]
    fn the_error_is_a_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(EventStoreError::Schema {
            found: 7,
            supported: 1,
        });
        assert_eq!(err.to_string(), "schema: found 7, supported 1");
    }

    #[test]
    fn failures_compare_by_value() {
        assert_eq!(
            EventStoreError::Conflict {
                expected: 1,
                actual: 2
            },
            EventStoreError::Conflict {
                expected: 1,
                actual: 2
            }
        );
        assert_ne!(
            EventStoreError::Conflict {
                expected: 1,
                actual: 2
            },
            EventStoreError::Conflict {
                expected: 1,
                actual: 3
            }
        );
    }
}
