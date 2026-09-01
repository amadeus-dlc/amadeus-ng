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

use core_command_domain::orchestration::IntentExecutionEvent;

use crate::workspace::{ReadModel, ResolvedPlan};

use super::catch_up_error::CatchUpError;
use super::global_seq_nr::GlobalSeqNr;
use super::journal_reader::JournalReader;
use super::projection_name::ProjectionName;
use super::projection_targets::ProjectionTargets;

/// ReadModelUpdater — チェックポイント以降のイベントをリードモデルへ流し込む差分関数。
#[derive(Debug)]
pub struct ReadModelUpdater<R> {
    journal_reader: R,
    projection: ProjectionName,
    targets: ProjectionTargets,
    /// 解決済み計画の控え。`Started` は 1 度しか書かれないので、一度引けば以後は使い回す。
    plan: Option<ResolvedPlan>,
}

impl<R: JournalReader> ReadModelUpdater<R> {
    /// 読み手・投影名・書込先から組む。
    pub const fn new(
        journal_reader: R,
        projection: ProjectionName,
        targets: ProjectionTargets,
    ) -> ReadModelUpdater<R> {
        ReadModelUpdater {
            journal_reader,
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
        let checkpoint = self.journal_reader.checkpoint(&self.projection).await?;
        let batch = self.journal_reader.events_after(checkpoint).await?;
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

        self.journal_reader
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
        let history = self.journal_reader.events_after(GlobalSeqNr::ZERO).await?;
        let mut started_ids = history
            .executions()
            .iter()
            .filter_map(|entry| match entry.event() {
                IntentExecutionEvent::Started(started) => Some(started.intent_id().clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        started_ids.dedup();
        // 単一 intent が本ループの契約である — 混在を黙って 1 つの計画で描かない
        // (CodeRabbit 指摘。intent ごとの振り分けは U7 の駆動設計と対で扱う)。
        if started_ids.len() > 1 {
            return Err(CatchUpError::MixedIntents);
        }
        let plan = started_ids
            .first()
            .and_then(|id| history.intents().iter().find(|intent| intent.id() == id))
            .map(ResolvedPlan::of)
            .ok_or(CatchUpError::PlanUnavailable)?;
        self.plan = Some(plan.clone());
        Ok(plan)
    }
}
