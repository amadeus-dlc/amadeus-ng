//! リードモデル更新（`ReadModelUpdater::update_read_models`）の失敗。
//!
//! 取得ループ `OrchestrationReadModelUpdater` と、構造化面・テスト契約・計画指紋・
//! Code Generation 開始可否・runtime-graph の各更新器が共有する失敗の型である。

use crate::read_tables::{ReadTablesError, UnsplittableSection};
use crate::workspace::{ProjectionError, StateFileReadError, StateFileWriteError};

use super::journal_read_error::JournalReadError;

/// リードモデル更新の失敗。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadModelUpdateError {
    /// 対象別への移行が必要な旧共有投影が公開済み。
    LegacyProjection {
        /// 旧投影の識別名。
        projection: String,
    },
    /// 計画の前提とファイル内容が一致しない。
    PublicationConflict {
        /// 内容が計画の前提と一致しない対象。
        path: std::path::PathBuf,
    },
    /// 公開計画の対象ファイルを読めない・書けない。
    PublicationIo {
        /// 操作に失敗した対象。
        path: std::path::PathBuf,
        /// OSが返した失敗分類。
        kind: std::io::ErrorKind,
    },
    /// ジャーナルの読取・チェックポイントの失敗。
    Read(JournalReadError),
    /// 投影核が描けなかった。
    Projection(ProjectionError),
    /// 状態ファイルを読めなかった（upstream 逐語の拒否文言を運ぶ）。
    StateFileRead(StateFileReadError),
    /// 状態ファイルを書けなかった。
    StateFileWrite(StateFileWriteError),
    /// メモリ層のファイルが**在るのに読めない**（b49）。
    ///
    /// 2 本が揃っていないのは正常（昇格の投影が要らない workspace）だが、在るのに読めない
    /// のは blocking である — 読めないまま進むと受領証だけが立って正本が古いままになる。
    MemoryFileRead {
        /// 読めなかったファイルのパス。
        path: String,
        /// OS が言った理由の分類。
        kind: std::io::ErrorKind,
    },
    /// メモリ層のファイルを書けなかった（b49）。
    MemoryFileWrite {
        /// 書けなかったファイルのパス。
        path: String,
        /// 失敗の材料（read-only バリア、または OS の I/O 文言）。
        detail: String,
    },
    /// 描くべき差分はあるのに、解決済み計画の材料がジャーナルに無い。
    ///
    /// 計画（表示属性・走査結果）の正本は intent 自身の誕生記録（`Created`）であり、どの
    /// intent かは実行の `Started` が指す（issue #56）。`Started` が無い・指された `Created`
    /// が無い、のどちらでも 1 行も描けない。ジャーナルが途中から切り落とされた兆候であり、
    /// 読み替えずに止める。
    PlanUnavailable,
    /// 差分を観測した後の全履歴取得が空になった。
    HistoryDisappeared,
    /// ジャーナルに**複数の intent** を指す実行が混在している。
    ///
    /// この取得ループは単一 intent の状態ファイル 1 面へ描く（`ProjectionTargets` は 1 組）。
    /// 別 intent の実行を同じ計画で描くと誤った表示属性が焼き込まれるため、混在は読み替えず
    /// 止める。intent ごとの書込先振り分けは合成ルート（U7）の駆動設計と対で扱う。
    ///
    /// **この契約は Markdown 面（系統 (1)）の話である** — 構造化面（`read_*` 表）は複数の
    /// intent も複数の実行もキーで自然に扱うので、混在そのものは構造化面の問題ではない。
    MixedIntents,
    /// 構造化投影核が歴史の切り落としを見つけた。
    ///
    /// ストリームの先頭に誕生記録が無い・実行が指す intent が履歴に無い、のどちらか。
    /// 読み替えて部分的な行を書くと、読取コマンドが「間違った答えが在る」を見ることに
    /// なるので止める。
    ReadTables(ReadTablesError),
    /// 参照入力 (memory 層) の規則ファイルが**在るのに読めない**。
    ///
    /// 無いのは正常 (規則未整備) だが、読めないのは blocking である — 規則を静かに落として
    /// 進むと、届く steering が痩せたまま誰も気づかない (02 §10)。運ぶのは材料だけ
    /// (どのファイルが・どう読めなかったか) で、利用者向けの文言は出す側が組む。
    SteeringRead {
        /// 読めなかった規則ファイルのパス (読み手が決めた綴り)。
        path: String,
        /// OS が言った理由の分類。
        kind: std::io::ErrorKind,
    },
    /// 参照入力を輸送目標未満へ刻めなかった (防御的)。
    ///
    /// 1 コードポイントが目標を超えるセクションでのみ成立する。1 フェーズでも刻めなければ
    /// steering の行を 1 つも書かずに止める。
    SteeringPack(UnsplittableSection),
}

impl core::fmt::Display for ReadModelUpdateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ReadModelUpdateError::LegacyProjection { projection } => write!(
                f,
                "legacy shared projection requires migration: {projection}"
            ),
            ReadModelUpdateError::PublicationConflict { path } => {
                write!(f, "publication conflict: {}", path.display())
            }
            ReadModelUpdateError::PublicationIo { path, kind } => {
                write!(f, "publication io: {kind:?} at {}", path.display())
            }
            ReadModelUpdateError::Read(inner) => write!(f, "read: {inner}"),
            ReadModelUpdateError::Projection(inner) => write!(f, "projection: {inner}"),
            ReadModelUpdateError::StateFileRead(inner) => {
                write!(f, "state file read: {}", inner.message())
            }
            ReadModelUpdateError::StateFileWrite(inner) => write!(f, "state file write: {inner:?}"),
            ReadModelUpdateError::MemoryFileRead { path, kind } => {
                write!(f, "memory file read: {kind:?} at {path}")
            }
            ReadModelUpdateError::MemoryFileWrite { path, detail } => {
                write!(f, "memory file write: {detail} at {path}")
            }
            ReadModelUpdateError::PlanUnavailable => f.write_str("plan unavailable"),
            ReadModelUpdateError::HistoryDisappeared => {
                f.write_str("history disappeared between reads")
            }
            ReadModelUpdateError::MixedIntents => f.write_str("mixed intents"),
            ReadModelUpdateError::ReadTables(inner) => write!(f, "read tables: {inner}"),
            ReadModelUpdateError::SteeringRead { path, kind } => {
                write!(f, "steering read: {kind:?} at {path}")
            }
            ReadModelUpdateError::SteeringPack(inner) => write!(f, "steering pack: {inner}"),
        }
    }
}

impl std::error::Error for ReadModelUpdateError {
    /// 内包した失敗へ連鎖する。
    ///
    /// **封筒は連鎖を切ってはならない** — 内包した失敗が自分の `source` に材料を載せている
    /// 場合、ここで `None` を返すとその材料はこの型で行き止まりになる（裁定 6 の帰結）。
    ///
    /// 連鎖するのは本物のエラー型を包む変種である。`StateFileRead` /
    /// `StateFileWrite` が包む型は `std::error::Error` ではなく**逐語文言を運ぶ値**であり
    /// （upstream 出力と 1 文字も違ってはならない文字列）、材料はこの型の `Display` が既に
    /// 描いている。`PlanUnavailable` / `MixedIntents` はループ自身の拒否で内包物を持たない。
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ReadModelUpdateError::Read(inner) => Some(inner),
            ReadModelUpdateError::Projection(inner) => Some(inner),
            ReadModelUpdateError::ReadTables(inner) => Some(inner),
            ReadModelUpdateError::SteeringPack(inner) => Some(inner),
            ReadModelUpdateError::PublicationConflict { .. }
            | ReadModelUpdateError::LegacyProjection { .. }
            | ReadModelUpdateError::PublicationIo { .. }
            | ReadModelUpdateError::StateFileRead(_)
            | ReadModelUpdateError::StateFileWrite(_)
            | ReadModelUpdateError::PlanUnavailable
            | ReadModelUpdateError::HistoryDisappeared
            | ReadModelUpdateError::MixedIntents
            | ReadModelUpdateError::SteeringRead { .. }
            | ReadModelUpdateError::MemoryFileRead { .. }
            | ReadModelUpdateError::MemoryFileWrite { .. } => None,
        }
    }
}

impl From<JournalReadError> for ReadModelUpdateError {
    fn from(inner: JournalReadError) -> ReadModelUpdateError {
        ReadModelUpdateError::Read(inner)
    }
}

impl From<ProjectionError> for ReadModelUpdateError {
    fn from(inner: ProjectionError) -> ReadModelUpdateError {
        ReadModelUpdateError::Projection(inner)
    }
}

impl From<ReadTablesError> for ReadModelUpdateError {
    fn from(inner: ReadTablesError) -> ReadModelUpdateError {
        ReadModelUpdateError::ReadTables(inner)
    }
}

impl From<UnsplittableSection> for ReadModelUpdateError {
    fn from(inner: UnsplittableSection) -> ReadModelUpdateError {
        ReadModelUpdateError::SteeringPack(inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::StateFileReadError;

    #[test]
    fn the_envelope_chains_to_the_failure_it_wraps() {
        // 封筒がここで連鎖を切ると、内包した失敗が自分の `source` に載せている材料へ
        // 辿り着けなくなる。読取・投影の変種は本物のエラー型を包む。
        let read: ReadModelUpdateError = JournalReadError::Io {
            kind: std::io::ErrorKind::WouldBlock,
            path: None,
        }
        .into();
        assert_eq!(
            std::error::Error::source(&read)
                .expect("読取の失敗へ連鎖する")
                .to_string(),
            "io: WouldBlock at -"
        );

        let projection: ReadModelUpdateError = ProjectionError::ParkSectionMissing.into();
        assert_eq!(
            std::error::Error::source(&projection)
                .expect("投影の失敗へ連鎖する")
                .to_string(),
            "park section missing"
        );
    }

    /// b49 のメモリ層 2 変種は材料を自分の `Display` に持ち、連鎖の先は無い。
    #[test]
    fn the_memory_layer_failures_render_their_material_and_end_the_chain() {
        let read = ReadModelUpdateError::MemoryFileRead {
            path: "memory/team.md".to_string(),
            kind: std::io::ErrorKind::IsADirectory,
        };
        assert_eq!(
            read.to_string(),
            "memory file read: IsADirectory at memory/team.md"
        );
        assert!(std::error::Error::source(&read).is_none());

        let write = ReadModelUpdateError::MemoryFileWrite {
            path: "memory/project.md".to_string(),
            detail: "read-only target".to_string(),
        };
        assert_eq!(
            write.to_string(),
            "memory file write: read-only target at memory/project.md"
        );
        assert!(std::error::Error::source(&write).is_none());
    }

    #[test]
    fn a_failure_that_owns_its_material_ends_the_chain() {
        // ループ自身の拒否は材料を自分の `Display` に持つ — 連鎖の先は無い。
        assert!(std::error::Error::source(&ReadModelUpdateError::MixedIntents).is_none());
        assert!(std::error::Error::source(&ReadModelUpdateError::PlanUnavailable).is_none());
    }

    #[test]
    fn every_read_model_update_failure_renders_its_material() {
        assert_eq!(
            ReadModelUpdateError::HistoryDisappeared.to_string(),
            "history disappeared between reads"
        );
        assert!(std::error::Error::source(&ReadModelUpdateError::HistoryDisappeared).is_none());
        let read: ReadModelUpdateError = JournalReadError::Io {
            kind: std::io::ErrorKind::WouldBlock,
            path: None,
        }
        .into();
        assert_eq!(read.to_string(), "read: io: WouldBlock at -");

        let projection: ReadModelUpdateError = ProjectionError::ParkSectionMissing.into();
        assert_eq!(projection.to_string(), "projection: park section missing");

        let state_read = ReadModelUpdateError::StateFileRead(StateFileReadError::new(
            "State file not found: /x",
        ));
        assert_eq!(
            state_read.to_string(),
            "state file read: State file not found: /x"
        );

        let state_write =
            ReadModelUpdateError::StateFileWrite(StateFileWriteError::ReadOnlyTarget {
                message: "state file is read-only: /x".to_string(),
            });
        assert!(
            state_write.to_string().starts_with("state file write: "),
            "実際: {state_write}"
        );

        assert_eq!(
            ReadModelUpdateError::PlanUnavailable.to_string(),
            "plan unavailable"
        );
        assert_eq!(
            ReadModelUpdateError::MixedIntents.to_string(),
            "mixed intents"
        );

        for (failure, message) in [
            (
                ReadModelUpdateError::PublicationConflict {
                    path: "/output".into(),
                },
                "publication conflict: /output",
            ),
            (
                ReadModelUpdateError::PublicationIo {
                    path: "/output".into(),
                    kind: std::io::ErrorKind::PermissionDenied,
                },
                "publication io: PermissionDenied at /output",
            ),
        ] {
            assert_eq!(failure.to_string(), message);
            assert!(std::error::Error::source(&failure).is_none());
        }
        let steering: ReadModelUpdateError =
            UnsplittableSection::new("memory/org.md".into()).into();
        assert_eq!(
            steering.to_string(),
            "steering pack: unsplittable section in memory/org.md"
        );
        assert_eq!(
            std::error::Error::source(&steering).unwrap().to_string(),
            "unsplittable section in memory/org.md"
        );

        let read_tables: ReadModelUpdateError = ReadTablesError::MissingGenesis {
            aggregate_id: "claude".to_string(),
        }
        .into();
        assert_eq!(
            read_tables.to_string(),
            "read tables: missing genesis for claude"
        );
        assert_eq!(
            std::error::Error::source(&read_tables)
                .expect("構造化投影の失敗へ連鎖する")
                .to_string(),
            "missing genesis for claude"
        );

        let steering_read = ReadModelUpdateError::SteeringRead {
            path: "memory/team.md".to_string(),
            kind: std::io::ErrorKind::PermissionDenied,
        };
        assert_eq!(
            steering_read.to_string(),
            "steering read: PermissionDenied at memory/team.md"
        );
        assert!(
            std::error::Error::source(&steering_read).is_none(),
            "材料は自分の Display に載っている"
        );

        let boxed: Box<dyn std::error::Error> = Box::new(projection);
        assert_eq!(boxed.to_string(), "projection: park section missing");
    }
}
