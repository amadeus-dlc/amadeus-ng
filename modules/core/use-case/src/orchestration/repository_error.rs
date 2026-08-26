//! `RepositoryError` — `WorkflowExecutionRepository` の失敗 (entities.md)。

use std::fmt;
use std::io::ErrorKind;
use std::path::PathBuf;

use core_domain::orchestration::IntentId;

use super::event_store_error::{CorruptCause, EventStoreError};

/// `WorkflowExecutionRepository` の失敗 (材料のみ — 逐語文言はアダプタ層)。
///
/// `EventStoreError` との違いは 2 点ある: (1) 集約識別子がドメイン型 (`IntentId`) であること、
/// (2) `Schema` / `CheckpointRegression` を持たないこと。どちらも Repository の面では
/// 「集約 1 つの再構成・永続化」しか語らないためで、下位の失敗は
/// [`RepositoryError::from_event_store`] が畳んで写す。
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

impl RepositoryError {
    /// 下位ポート (`EventStore` / `JournalReader`) の失敗を Repository 面へ写す (BR1.5)。
    ///
    /// `intent_id` は**呼出文脈の集約識別子**。`EventStoreError` は集約識別子を生文字列でしか
    /// 持たないため、写すときにドメイン型へ格上げする必要がある: 行の識別子が
    /// `IntentId` として妥当ならそれを、妥当でない (= 行そのものが壊れている) なら呼出文脈の
    /// 識別子を使う。
    ///
    /// `Schema` は「このストアを読めない」= 復号できない状態なので `Corrupt(SchemaVersion)` に、
    /// `CheckpointRegression` は投影 (U4) の面の失敗であり Repository の 2 メソッドからは
    /// 到達しないため `Corrupt(InvariantViolation)` に畳む (投影名の材料は Repository の
    /// `Corrupt` に置き場が無いので落ちる — entities.md の 4 変種を増やさないため)。
    #[must_use]
    pub fn from_event_store(error: EventStoreError, intent_id: &IntentId) -> RepositoryError {
        match error {
            EventStoreError::Conflict { expected, actual } => {
                RepositoryError::Conflict { expected, actual }
            }
            EventStoreError::Io { kind, path } => RepositoryError::Io { kind, path },
            EventStoreError::Corrupt {
                aggregate_id,
                seq_nr,
                cause,
            } => RepositoryError::Corrupt {
                aggregate_id: IntentId::parse(&aggregate_id).unwrap_or_else(|_| intent_id.clone()),
                seq_nr,
                cause,
            },
            EventStoreError::Schema { .. } => RepositoryError::Corrupt {
                aggregate_id: intent_id.clone(),
                seq_nr: None,
                cause: CorruptCause::SchemaVersion,
            },
            EventStoreError::CheckpointRegression { .. } => RepositoryError::Corrupt {
                aggregate_id: intent_id.clone(),
                seq_nr: None,
                cause: CorruptCause::InvariantViolation,
            },
        }
    }
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
    use crate::orchestration::{CorruptCause, EventStoreError, GlobalSeqNr, ProjectionName};
    use core_domain::orchestration::IntentId;
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
    fn the_conflict_and_io_variants_map_across_unchanged() {
        assert_eq!(
            RepositoryError::from_event_store(
                EventStoreError::Conflict {
                    expected: 1,
                    actual: 2
                },
                &intent()
            ),
            RepositoryError::Conflict {
                expected: 1,
                actual: 2
            }
        );
        assert_eq!(
            RepositoryError::from_event_store(
                EventStoreError::Io {
                    kind: ErrorKind::WouldBlock,
                    path: None
                },
                &intent()
            ),
            RepositoryError::Io {
                kind: ErrorKind::WouldBlock,
                path: None
            }
        );
    }

    #[test]
    fn the_corrupt_variant_keeps_its_cause_and_promotes_the_carried_aggregate_id() {
        assert_eq!(
            RepositoryError::from_event_store(
                EventStoreError::Corrupt {
                    aggregate_id: RAW_ID.to_string(),
                    seq_nr: Some(4),
                    cause: CorruptCause::UnknownEventType,
                },
                &intent()
            ),
            RepositoryError::Corrupt {
                aggregate_id: intent(),
                seq_nr: Some(4),
                cause: CorruptCause::UnknownEventType,
            }
        );
    }

    #[test]
    fn an_unparsable_carried_aggregate_id_falls_back_to_the_call_context() {
        assert_eq!(
            RepositoryError::from_event_store(
                EventStoreError::Corrupt {
                    aggregate_id: "not-a-uuid".to_string(),
                    seq_nr: None,
                    cause: CorruptCause::UndecodablePayload,
                },
                &intent()
            ),
            RepositoryError::Corrupt {
                aggregate_id: intent(),
                seq_nr: None,
                cause: CorruptCause::UndecodablePayload,
            }
        );
    }

    #[test]
    fn the_schema_failure_becomes_a_corrupt_schema_version() {
        assert_eq!(
            RepositoryError::from_event_store(
                EventStoreError::Schema {
                    found: 2,
                    supported: 1
                },
                &intent()
            ),
            RepositoryError::Corrupt {
                aggregate_id: intent(),
                seq_nr: None,
                cause: CorruptCause::SchemaVersion,
            }
        );
    }

    #[test]
    fn the_checkpoint_regression_is_not_reachable_from_the_repository_face_and_folds_into_corrupt()
    {
        assert_eq!(
            RepositoryError::from_event_store(
                EventStoreError::CheckpointRegression {
                    projection: ProjectionName::parse("state-file").unwrap(),
                    current: GlobalSeqNr::new(3),
                    requested: GlobalSeqNr::new(1),
                },
                &intent()
            ),
            RepositoryError::Corrupt {
                aggregate_id: intent(),
                seq_nr: None,
                cause: CorruptCause::InvariantViolation,
            }
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
