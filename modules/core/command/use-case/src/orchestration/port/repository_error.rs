//! `RepositoryError<Id>` — Repository ポート共通の失敗 (材料のみ)。

use std::error::Error;
use std::fmt;
use std::io::ErrorKind;
use std::path::PathBuf;

/// Repository ポートの失敗 (材料のみ — 逐語文言はアダプタ層)。
///
/// **リポジトリごとに別のエラー型を作らない** (オーナー裁定 2026-08-30) — 変種の形は面に
/// よらず同じで、違うのは集約 ID の型だけである。ID 型だけが違う複製 (旧
/// `IntentRepositoryError`) を面ごとに増やす設計は廃止し、型引数 `Id` で面を区別する。
/// 読取専用のポートでも `Conflict` が型上は構成可能になるが、「構成不能を型で語る」精密さ
/// より統一が勝るという裁定である。
///
/// `Corrupt` の**分類はポート契約に載せない** (裁定 6 / u5-report-use-case/decisions-1.md —
/// エラーは契約の一部であり、内部実装がバレる情報を含めない)。原因は標準ライブラリの
/// 原因連鎖 (`Error::source`) でアダプタ私有のエラー型を運び、契約は「壊れていた」としか
/// 約束しない。診断表示 (caused by: ...) は連鎖から残る。
///
/// 下位のイベントストア (本家 event-store-adapter-rs) の失敗を本型へ写すのは Gateway 実装の
/// 責務であり、ユースケース層は本家のエラー型を知らない (ADR-010)。
///
/// `source` が比較不能なため `PartialEq` は実装しない (裁定 6 で受容済み) — テストは
/// `matches!` と `source` の文字列確認で判定する。
#[derive(Debug)]
pub enum RepositoryError<Id> {
    /// この識別子の集約がストアに無い。
    NotFound {
        /// 探した集約識別子。
        id: Id,
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
    /// ストアの記録が壊れている (部分データは返さない — BR1.2)。
    Corrupt {
        /// 対象の集約識別子。
        id: Id,
        /// 該当行の `seq_nr` (行が特定できない場合は `None`)。
        seq_nr: Option<usize>,
        /// アダプタ私有の原因 (契約は型を約束しない — 診断表示だけを運ぶ)。
        source: Box<dyn Error + Send + Sync>,
    },
}

impl<Id: fmt::Display> fmt::Display for RepositoryError<Id> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepositoryError::NotFound { id } => write!(f, "not found: {id}"),
            RepositoryError::Conflict { expected, actual } => {
                write!(f, "conflict: expected {expected}, actual {actual}")
            }
            RepositoryError::Io { kind, path } => write!(
                f,
                "io: {kind:?} at {}",
                path.as_ref()
                    .map_or_else(|| "-".to_string(), |p| p.display().to_string())
            ),
            RepositoryError::Corrupt { id, seq_nr, .. } => write!(
                f,
                "corrupt: aggregate {id}, seq_nr {}",
                seq_nr.map_or_else(|| "-".to_string(), |n| n.to_string())
            ),
        }
    }
}

impl<Id: fmt::Display + fmt::Debug> Error for RepositoryError<Id> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            RepositoryError::Corrupt { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use core_command_domain::orchestration::{IntentExecutionId, IntentId};

    use super::*;

    fn execution_id() -> IntentExecutionId {
        IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap()
    }

    fn intent_id() -> IntentId {
        IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap()
    }

    #[derive(Debug)]
    struct FakeCause;

    impl fmt::Display for FakeCause {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("fake cause")
        }
    }

    impl Error for FakeCause {}

    #[test]
    fn every_variant_renders_its_material() {
        assert_eq!(
            RepositoryError::NotFound { id: execution_id() }.to_string(),
            "not found: 0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000"
        );
        assert_eq!(
            RepositoryError::<IntentExecutionId>::Conflict {
                expected: 3,
                actual: 5,
            }
            .to_string(),
            "conflict: expected 3, actual 5"
        );
        assert_eq!(
            RepositoryError::<IntentExecutionId>::Io {
                kind: ErrorKind::PermissionDenied,
                path: Some(PathBuf::from("/tmp/store.db")),
            }
            .to_string(),
            "io: PermissionDenied at /tmp/store.db"
        );
        assert_eq!(
            RepositoryError::Corrupt {
                id: execution_id(),
                seq_nr: Some(4),
                source: Box::new(FakeCause),
            }
            .to_string(),
            "corrupt: aggregate 0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000, seq_nr 4"
        );
    }

    #[test]
    fn the_same_shape_serves_every_aggregate_id() {
        // ジェネリック 1 本で面を区別する — intent 面にも同じ変種の形がそのまま使える。
        assert_eq!(
            RepositoryError::NotFound { id: intent_id() }.to_string(),
            "not found: 01a02785-1bd8-76eb-aeea-5aa303ebd5b6"
        );
    }

    #[test]
    fn the_corrupt_cause_travels_the_source_chain() {
        // 分類は契約に載らない (裁定 6) — 原因は `Error::source` の連鎖で診断表示だけを運ぶ。
        let error = RepositoryError::Corrupt {
            id: execution_id(),
            seq_nr: None,
            source: Box::new(FakeCause),
        };
        let source = Error::source(&error).expect("Corrupt は原因を連鎖する");
        assert_eq!(source.to_string(), "fake cause");
        assert!(Error::source(&RepositoryError::NotFound { id: execution_id() }).is_none());
    }
}
