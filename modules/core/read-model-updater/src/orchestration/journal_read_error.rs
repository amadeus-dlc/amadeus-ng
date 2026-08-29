//! `JournalReadError` — [`JournalReader`] の失敗 (entities.md)。
//!
//! 集約の永続化 (書込・再構成) の失敗はコマンド側の `RepositoryError` が持つ。本型が語るのは
//! 「ジャーナルの横断読取」と「投影チェックポイント」の面だけである — 楽観 version の競合が
//! 変種に無いのはそのためで、競合は書込の面 (コマンド側) の話である。RMU の依存はコマンド側の
//! うち `core-command-domain` だけで `core-command-use-case` を含まないので、相手の型への
//! rustdoc リンクはそもそも張れない (`coding-rules/cqrs-boundaries.md` — 中間である RMU が
//! 依存してよいのは投影核の入口に要るものだけ、という自制でもある)。
//!
//! [`JournalReader`]: super::journal_reader::JournalReader

use std::fmt;
use std::io::ErrorKind;
use std::path::PathBuf;

use super::corrupt_cause::CorruptCause;
use super::global_seq_nr::GlobalSeqNr;
use super::projection_name::ProjectionName;

/// `JournalReader` の失敗 (材料のみ — 利用者向けの逐語文言はアダプタ層の
/// message-catalog が組み立てる。`coding-rules/error-handling.md`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalReadError {
    /// ストア I/O の失敗。`ErrorKind` を保持する (監査 C24)。書込ロック待ちの超過は
    /// `ErrorKind::WouldBlock` に写す (NFR3.5)。
    Io {
        /// OS / ドライバ由来の分類。
        kind: ErrorKind,
        /// 対象パス (分からない場合は `None`)。
        path: Option<PathBuf>,
    },
    /// 行を読めたがドメインへ写せない (復号不能・値が列に収まらない)。
    ///
    /// `aggregate_id` は**行に入っていた生文字列**。ドメイン型ではないのは、破損した行の
    /// 識別子が `IntentId` として妥当とは限らないためである。
    Corrupt {
        /// 行が名乗っていた集約識別子 (生文字列)。
        aggregate_id: String,
        /// 該当行の `seq_nr` (行が特定できない場合は `None`)。
        seq_nr: Option<usize>,
        /// 原因の分類。
        cause: CorruptCause,
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

/// `Option<PathBuf>` を材料として描く (欠落は `-`)。
fn render_path(path: Option<&PathBuf>) -> String {
    path.map_or_else(|| "-".to_string(), |p| p.display().to_string())
}

/// 欠落しうる `seq_nr` を材料として描く。
fn render_seq_nr(seq_nr: Option<usize>) -> String {
    seq_nr.map_or_else(|| "-".to_string(), |n| n.to_string())
}

impl fmt::Display for JournalReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JournalReadError::Io { kind, path } => {
                write!(f, "io: {kind:?} at {}", render_path(path.as_ref()))
            }
            JournalReadError::Corrupt {
                aggregate_id,
                seq_nr,
                cause,
            } => write!(
                f,
                "corrupt: aggregate {aggregate_id}, seq_nr {}, cause {cause}",
                render_seq_nr(*seq_nr)
            ),
            JournalReadError::CheckpointRegression {
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

impl std::error::Error for JournalReadError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::{CorruptCause, GlobalSeqNr, ProjectionName};
    use std::io::ErrorKind;
    use std::path::PathBuf;

    fn projection() -> ProjectionName {
        ProjectionName::parse("state-file").unwrap()
    }

    #[test]
    fn the_io_failure_carries_the_kind_and_the_path() {
        let err = JournalReadError::Io {
            kind: ErrorKind::NotFound,
            path: Some(PathBuf::from("/tmp/.aidlc-store.sqlite")),
        };
        assert_eq!(err.to_string(), "io: NotFound at /tmp/.aidlc-store.sqlite");
        let without_path = JournalReadError::Io {
            kind: ErrorKind::WouldBlock,
            path: None,
        };
        assert_eq!(without_path.to_string(), "io: WouldBlock at -");
    }

    #[test]
    fn the_corrupt_failure_carries_the_aggregate_the_sequence_and_the_cause() {
        let err = JournalReadError::Corrupt {
            aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
            seq_nr: Some(4),
            cause: CorruptCause::UndecodablePayload,
        };
        assert_eq!(
            err.to_string(),
            "corrupt: aggregate 01a02785-1bd8-76eb-aeea-5aa303ebd5b6, seq_nr 4, cause undecodable payload"
        );
        let without_seq = JournalReadError::Corrupt {
            aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
            seq_nr: None,
            cause: CorruptCause::CheckpointAnchorMismatch,
        };
        assert_eq!(
            without_seq.to_string(),
            "corrupt: aggregate 01a02785-1bd8-76eb-aeea-5aa303ebd5b6, seq_nr -, cause checkpoint anchor mismatch"
        );
    }

    #[test]
    fn the_checkpoint_regression_carries_the_projection_and_both_positions() {
        let err = JournalReadError::CheckpointRegression {
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
    fn the_error_is_a_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(JournalReadError::Io {
            kind: ErrorKind::Other,
            path: None,
        });
        assert_eq!(err.to_string(), "io: Other at -");
    }

    #[test]
    fn failures_compare_by_value() {
        assert_eq!(
            JournalReadError::Io {
                kind: ErrorKind::WouldBlock,
                path: None
            },
            JournalReadError::Io {
                kind: ErrorKind::WouldBlock,
                path: None
            }
        );
        assert_ne!(
            JournalReadError::Io {
                kind: ErrorKind::WouldBlock,
                path: None
            },
            JournalReadError::Io {
                kind: ErrorKind::NotFound,
                path: None
            }
        );
    }
}
