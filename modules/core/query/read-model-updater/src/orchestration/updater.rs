//! **取得ループ** — RMU の上側の層（2026-08-28 裁定 / `coding-rules/cqrs-boundaries.md`）。
//!
//! ```text
//! checkpoint 読取 → events_after で差分取得 → 純粋投影核 → リードモデルを書く → advance_checkpoint
//! ```
//!
//! SQLite にはストリームが無いので、AWS 版 RMU が Streams から**受信する**のと同じ役割を、
//! ここでは**自分で引く**形で果たす。イベントを運ぶのは RMU 自身であり、合成ルート（U7）は
//! これを**起動するだけ**である — 駆動ループを合成ルートへ置くと、バッチ・チェックポイント
//! 単調性・エラー処理という実ロジックがカバレッジ除外領域へ落ちてしまう。
//!
//! 投影核（`workspace::project`）はこの層を知らない。知っているのは片方向である。

use std::path::{Path, PathBuf};

use crate::workspace::{
    AuditShardWriteError, ProjectionError, ReadModel, StateFileReadError, StateFileWriteError,
};

use super::global_seq_nr::GlobalSeqNr;
use super::journal_read_error::JournalReadError;
use super::journal_reader::JournalReader;
use super::projection_name::ProjectionName;

/// キャッチアップの失敗。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatchUpError {
    /// ジャーナルの読取・チェックポイントの失敗。
    Read(JournalReadError),
    /// 投影核が描けなかった。
    Projection(ProjectionError),
    /// 状態ファイルを読めなかった（upstream 逐語の拒否文言を運ぶ）。
    StateFileRead(StateFileReadError),
    /// 状態ファイルを書けなかった。
    StateFileWrite(StateFileWriteError),
    /// 監査シャードへ追記できなかった。
    AuditShardWrite(AuditShardWriteError),
}

impl core::fmt::Display for CatchUpError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CatchUpError::Read(inner) => write!(f, "read: {inner}"),
            CatchUpError::Projection(inner) => write!(f, "projection: {inner}"),
            CatchUpError::StateFileRead(inner) => {
                write!(f, "state file read: {}", inner.message())
            }
            CatchUpError::StateFileWrite(inner) => write!(f, "state file write: {inner:?}"),
            CatchUpError::AuditShardWrite(inner) => write!(f, "audit shard write: {inner}"),
        }
    }
}

impl std::error::Error for CatchUpError {}

impl From<JournalReadError> for CatchUpError {
    fn from(inner: JournalReadError) -> CatchUpError {
        CatchUpError::Read(inner)
    }
}

impl From<ProjectionError> for CatchUpError {
    fn from(inner: ProjectionError) -> CatchUpError {
        CatchUpError::Projection(inner)
    }
}

/// 投影の書込先 2 面の場所。
///
/// 生の `PathBuf` を 2 本ばらばらに引き回さないための束である — 片方だけ差し替わった
/// 取り合わせ（別 intent の状態ファイルと別 clone のシャード）を構成できなくする。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionTargets {
    state_file: PathBuf,
    audit_shard: PathBuf,
}

impl ProjectionTargets {
    /// 状態ファイルと監査シャードの場所から組む。
    #[must_use]
    pub fn new(
        state_file: impl Into<PathBuf>,
        audit_shard: impl Into<PathBuf>,
    ) -> ProjectionTargets {
        ProjectionTargets {
            state_file: state_file.into(),
            audit_shard: audit_shard.into(),
        }
    }

    /// 状態ファイル（`aidlc-state.md`）の場所。
    #[must_use]
    pub fn state_file(&self) -> &Path {
        &self.state_file
    }

    /// 監査シャード（`<record>/audit/<host>-<clone>.md`）の場所。
    #[must_use]
    pub fn audit_shard(&self) -> &Path {
        &self.audit_shard
    }
}

/// ReadModelUpdater — チェックポイント以降のイベントをリードモデルへ流し込む差分関数。
#[derive(Debug)]
pub struct ReadModelUpdater<R> {
    reader: R,
    projection: ProjectionName,
    targets: ProjectionTargets,
}

impl<R: JournalReader> ReadModelUpdater<R> {
    /// 読み手・投影名・書込先から組む。
    pub const fn new(
        reader: R,
        projection: ProjectionName,
        targets: ProjectionTargets,
    ) -> ReadModelUpdater<R> {
        ReadModelUpdater {
            reader,
            projection,
            targets,
        }
    }

    /// 書込先の場所。
    #[must_use]
    pub const fn targets(&self) -> &ProjectionTargets {
        &self.targets
    }

    /// チェックポイント以降を読んで描き、チェックポイントを進める。
    ///
    /// 戻り値は前進後のチェックポイントである。差分が空なら**何も書かず**現在値を返す。
    ///
    /// # 書いてから進める（at-least-once）
    ///
    /// リードモデルをディスクへ落としてからチェックポイントを進める。逆順にすると、
    /// 書込の直前で落ちたときに監査行が**永久に失われる** — 台帳にとって欠落は重複より重い。
    /// 書込後・前進前に落ちた場合は同じ差分を再実行することになり、状態ファイルは同じ位置へ
    /// 落ち着く（冪等）が、監査シャードには同じブロックがもう一度並ぶ。この非対称は
    /// 「欠落しない」ことと引き換えに受け入れている。
    ///
    /// # Errors
    ///
    /// ジャーナルの読取・チェックポイントの失敗（`Read`）、投影核が描けなかった
    /// （`Projection`）、状態ファイルを読めない（`StateFileRead`）・書けない
    /// （`StateFileWrite`）、監査シャードへ追記できない（`AuditShardWrite`）。
    pub async fn catch_up(&mut self) -> Result<GlobalSeqNr, CatchUpError> {
        let checkpoint = self.reader.checkpoint(&self.projection).await?;
        let entries = self.reader.events_after(checkpoint).await?;
        let Some(last) = entries
            .last()
            .map(super::journal_entry::JournalEntry::global_seq)
        else {
            return Ok(checkpoint);
        };

        let state = crate::workspace::read_state_file(self.targets.state_file())
            .map_err(CatchUpError::StateFileRead)?;
        let mut read_model = ReadModel::new(state);
        crate::workspace::project(&entries, &mut read_model)?;

        crate::workspace::write_state_file(self.targets.state_file(), read_model.state())
            .map_err(CatchUpError::StateFileWrite)?;
        crate::workspace::append_audit_shard(
            self.targets.audit_shard(),
            read_model.appended_audit(),
        )
        .map_err(CatchUpError::AuditShardWrite)?;

        self.reader
            .advance_checkpoint(&self.projection, last)
            .await?;
        Ok(last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{AuditShardWriteError, StateFileReadError};

    #[test]
    fn every_catch_up_failure_renders_its_material() {
        let read: CatchUpError = JournalReadError::Io {
            kind: std::io::ErrorKind::WouldBlock,
            path: None,
        }
        .into();
        assert_eq!(read.to_string(), "read: io: WouldBlock at -");

        let projection: CatchUpError = ProjectionError::ScopeUnknown.into();
        assert_eq!(projection.to_string(), "projection: scope unknown");

        let state_read =
            CatchUpError::StateFileRead(StateFileReadError::new("State file not found: /x"));
        assert_eq!(
            state_read.to_string(),
            "state file read: State file not found: /x"
        );

        let state_write = CatchUpError::StateFileWrite(StateFileWriteError::ReadOnlyTarget {
            message: "state file is read-only: /x".to_string(),
        });
        assert!(
            state_write.to_string().starts_with("state file write: "),
            "実際: {state_write}"
        );

        let shard_write = CatchUpError::AuditShardWrite(AuditShardWriteError::Io {
            kind: std::io::ErrorKind::PermissionDenied,
        });
        assert_eq!(
            shard_write.to_string(),
            "audit shard write: io: PermissionDenied"
        );

        let boxed: Box<dyn std::error::Error> = Box::new(projection);
        assert_eq!(boxed.to_string(), "projection: scope unknown");
    }

    #[test]
    fn the_targets_keep_both_paths_together() {
        let targets = ProjectionTargets::new("/w/aidlc-state.md", "/w/audit/host-abcd1234.md");
        assert_eq!(targets.state_file(), Path::new("/w/aidlc-state.md"));
        assert_eq!(
            targets.audit_shard(),
            Path::new("/w/audit/host-abcd1234.md")
        );
        assert_eq!(
            targets,
            ProjectionTargets::new("/w/aidlc-state.md", "/w/audit/host-abcd1234.md")
        );
    }
}
