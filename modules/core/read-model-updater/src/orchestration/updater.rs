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

use core_command_domain::orchestration::IntentExecutionEvent;

use crate::workspace::{
    AuditShardWriteError, ProjectionError, ReadModel, ResolvedPlan, StateFileReadError,
    StateFileWriteError,
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
    /// 描くべき差分はあるのに、解決済み計画の材料がジャーナルに無い。
    ///
    /// 計画（表示属性・走査結果）の正本は intent 自身の誕生記録（`Created`）であり、どの
    /// intent かは実行の `Started` が指す（issue #56）。`Started` が無い・指された `Created`
    /// が無い、のどちらでも 1 行も描けない。ジャーナルが途中から切り落とされた兆候であり、
    /// 読み替えずに止める。
    PlanUnavailable,
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
            CatchUpError::PlanUnavailable => f.write_str("plan unavailable"),
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
    /// 解決済み計画の控え。`Started` は 1 度しか書かれないので、一度引けば以後は使い回す。
    plan: Option<ResolvedPlan>,
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
            plan: None,
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
        let batch = self.reader.events_after(checkpoint).await?;
        let Some(last) = batch.scanned_to() else {
            return Ok(checkpoint);
        };

        // 実行のイベントがあるときだけ描く。intent の行しか無いバッチは書くものが無い —
        // それでもチェックポイントは走査済み位置まで進める（intent 行を毎回再走査しない。
        // issue #56 申し送りの解消）。
        if !batch.executions().is_empty() {
            let plan = self.resolve_plan().await?;
            let state = crate::workspace::read_state_file(self.targets.state_file())
                .map_err(CatchUpError::StateFileRead)?;
            let mut read_model = ReadModel::new(state);
            crate::workspace::project(batch.executions(), &plan, &mut read_model)?;

            crate::workspace::write_state_file(self.targets.state_file(), read_model.state())
                .map_err(CatchUpError::StateFileWrite)?;
            crate::workspace::append_audit_shard(
                self.targets.audit_shard(),
                read_model.appended_audit(),
            )
            .map_err(CatchUpError::AuditShardWrite)?;
        }

        self.reader
            .advance_checkpoint(&self.projection, last)
            .await?;
        Ok(last)
    }

    /// 解決済み計画を得る（初回だけジャーナルの先頭から引く）。
    ///
    /// 計画（表示属性・走査結果）の正本は intent 自身の誕生記録（`Created`）であり、どの
    /// intent かは実行の `Started` が指す（issue #56）。差分投影のバッチにその 2 行が入って
    /// いるとは限らない。取ってくるのは**この層の仕事**である — 投影核は計画を受け取るだけで、
    /// どこから来たかを知らない（二層構造）。
    ///
    /// どちらもワークフローごとに 1 度しか書かれないので、一度引けば控えを使い回す。
    async fn resolve_plan(&mut self) -> Result<ResolvedPlan, CatchUpError> {
        if let Some(plan) = &self.plan {
            return Ok(plan.clone());
        }
        let history = self.reader.events_after(GlobalSeqNr::ZERO).await?;
        let intent_id = history
            .executions()
            .iter()
            .find_map(|entry| match entry.event() {
                IntentExecutionEvent::Started(started) => Some(started.intent_id().clone()),
                _ => None,
            });
        let plan = intent_id
            .and_then(|id| history.intents().iter().find(|intent| intent.id() == &id))
            .map(ResolvedPlan::of)
            .ok_or(CatchUpError::PlanUnavailable)?;
        self.plan = Some(plan.clone());
        Ok(plan)
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

        let projection: CatchUpError = ProjectionError::ParkSectionMissing.into();
        assert_eq!(projection.to_string(), "projection: park section missing");

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

        assert_eq!(
            CatchUpError::PlanUnavailable.to_string(),
            "plan unavailable"
        );

        let shard_write = CatchUpError::AuditShardWrite(AuditShardWriteError::Io {
            kind: std::io::ErrorKind::PermissionDenied,
        });
        assert_eq!(
            shard_write.to_string(),
            "audit shard write: io: PermissionDenied"
        );

        let boxed: Box<dyn std::error::Error> = Box::new(projection);
        assert_eq!(boxed.to_string(), "projection: park section missing");
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
