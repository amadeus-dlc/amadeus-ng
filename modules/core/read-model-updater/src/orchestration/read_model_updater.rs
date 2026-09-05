//! **取得ループ** — RMU の上側の層（2026-08-28 裁定 / `coding-rules/cqrs-boundaries.md`）。
//!
//! ```text
//! 参照入力の読取 → source_digest 比較 → (変化時のみ) replace_steering   ← 別 Tx
//! checkpoint 読取 → 差分の探り → 全履歴 1 回の読取 → 純粋投影核 → リードモデルを書く → advance_checkpoint
//! ```
//!
//! 1 行目が参照入力（memory 層の規則ファイル）の面、2 行目がジャーナルの面である。規則の
//! 編集はイベントを伴わないので、ジャーナル差分が空でも 1 行目は毎回走る。
//!
//! SQLite にはストリームが無いので、AWS 版 RMU が Streams から**受信する**のと同じ役割を、
//! ここでは**自分で引く**形で果たす。イベントを運ぶのは RMU 自身であり、合成ルート（U7）は
//! これを**起動するだけ**である — 駆動ループを合成ルートへ置くと、バッチ・チェックポイント
//! 単調性・エラー処理という実ロジックがカバレッジ除外領域へ落ちてしまう。
//!
//! 投影核（`workspace::project`）はこの層を知らない。知っているのは片方向である。

use core_command_domain::orchestration::IntentExecutionEvent;

use crate::read_tables::{ReadTables, SteeringTables};
use crate::workspace::{ReadModel, ResolvedPlan};

use super::catch_up_error::CatchUpError;
use super::global_seq_nr::GlobalSeqNr;
use super::journal_batch::JournalBatch;
use super::journal_reader::JournalReader;
use super::projection_name::ProjectionName;
use super::projection_targets::ProjectionTargets;
use super::steering_source::SteeringSource;
use super::{PublicationBatch, PublicationFile};

/// ReadModelUpdater — チェックポイント以降のイベントをリードモデルへ流し込む差分関数。
#[derive(Debug)]
pub struct ReadModelUpdater<R> {
    journal_reader: R,
    projection: ProjectionName,
    targets: ProjectionTargets,
    /// 参照入力 (memory 層) の読取先。ジャーナルとは別の入口である。
    steering: SteeringSource,
    /// 解決済み計画の控え。`Started` は 1 度しか書かれないので、一度引けば以後は使い回す。
    plan: Option<ResolvedPlan>,
}

impl<R: JournalReader> ReadModelUpdater<R> {
    /// 読み手・投影名・書込先・参照入力の読取先から組む。
    pub const fn new(
        journal_reader: R,
        projection: ProjectionName,
        targets: ProjectionTargets,
        steering: SteeringSource,
    ) -> ReadModelUpdater<R> {
        ReadModelUpdater {
            journal_reader,
            projection,
            targets,
            steering,
            plan: None,
        }
    }

    /// 書込先の場所。
    #[must_use]
    pub const fn targets(&self) -> &ProjectionTargets {
        &self.targets
    }

    /// 参照入力の読取先。
    #[must_use]
    pub const fn steering(&self) -> &SteeringSource {
        &self.steering
    }

    /// Markdown の投影先がまだ無い初回起動で、構造化面だけを最新化する。
    ///
    /// fresh workspace では intent の記録ディレクトリも `aidlc-state.md` もまだ存在しないが、
    /// 最初の `next` は定義・scope・費用の行を読めなければ `intent-create` を名指せない。
    /// その初回起動だけがこの入口を使う。通常の実行では [`Self::catch_up`] が Markdown 面と
    /// 構造化面を同じ履歴断面から一緒に描く。
    ///
    /// # Errors
    ///
    /// ジャーナル・チェックポイントの読取、構造化投影、またはチェックポイントと行の
    /// 同一トランザクション更新に失敗した場合。
    pub async fn catch_up_structured(
        journal_reader: &mut R,
        projection: &ProjectionName,
    ) -> Result<GlobalSeqNr, CatchUpError> {
        let checkpoint = journal_reader.checkpoint(projection).await?;
        if journal_reader
            .events_after(checkpoint)
            .await?
            .scanned_to()
            .is_none()
        {
            return Ok(checkpoint);
        }

        let history = journal_reader.events_after(GlobalSeqNr::ZERO).await?;
        let Some(last) = history.scanned_to() else {
            return Ok(checkpoint);
        };
        let tables = ReadTables::project(&history)?;
        journal_reader
            .advance_checkpoint(projection, last, &tables)
            .await?;
        Ok(last)
    }

    /// チェックポイント以降を読んで描き、チェックポイントを進める。
    ///
    /// 戻り値は前進後のチェックポイントである。差分が空なら**何も書かず**現在値を返す。
    ///
    /// # 計画を保存し、公開してから進める
    ///
    /// リードモデルをディスクへ落としてからチェックポイントを進める。逆順にすると、
    /// 書込の直前で落ちたときに監査行が**永久に失われる**。その隙間を閉じるため、
    /// 書込前後のバイトを持つ公開計画を先に耐久化する。書込後・前進前の停止からは
    /// 保存済み計画と現物を照合し、反映済みを追記せず未反映部分だけを完了する。
    ///
    /// # 2 系統を 1 回で描く
    ///
    /// キャッチアップ 1 回で Markdown 面（系統 (1) — `aidlc-state.md` と監査シャード）と
    /// 構造化面（系統 (2) — SQLite の `read_*` 表）の両方を描く。構造化面は**全履歴からの
    /// 再計算**なので入力は全履歴であり、Markdown 面の差分もその履歴から
    /// 「チェックポイントより後の行」として切り出す — **描く材料は 1 回の読取に揃える**。
    /// 2 つの読取に跨がると、その間に入った書込のぶんだけ両面の断面がずれ、行の `as_of` が
    /// チェックポイントを追い越す。差分読取は「進む先があるか」の探りにだけ使う。
    /// 公開計画の確定とチェックポイントの前進は `publish` の中で1トランザクションに
    /// 閉じる。共有構造化面が別投影によって既に新しい場合は、その面を維持する。
    ///
    /// # 参照入力はジャーナルより先に見る
    ///
    /// steering の面（`read_steering_*`）の材料は**人が編集するファイル**であって
    /// ジャーナルではない。規則を直してもイベントは 1 件も増えないので、ジャーナル差分が
    /// 空でも参照入力は見る — したがって差分の探りより**前**に置く。読むのは毎回だが、
    /// 書き替えるのは `source_digest` が動いたときだけであり、その比較と差し替えは
    /// チェックポイントとは別のトランザクションである（設計 §3）。
    ///
    /// # Errors
    ///
    /// ジャーナルの読取・チェックポイントの失敗（`Read`）、投影核が描けなかった
    /// （`Projection`）、状態ファイルを読めない（`StateFileRead`）・書けない
    /// （`StateFileWrite`）、監査シャードへ追記できない（`AuditShardWrite`）、構造化投影核が
    /// 歴史の切り落としを見つけた（`ReadTables`）、参照入力の規則ファイルが在るのに読めない
    /// （`SteeringRead`）・刻めない（`SteeringPack`）。
    pub async fn catch_up(&mut self) -> Result<GlobalSeqNr, CatchUpError> {
        if let Some(batch) = self
            .journal_reader
            .pending_publication(&self.projection)
            .await?
        {
            if !batch.matches_targets(&self.targets) {
                return Err(CatchUpError::PublicationConflict {
                    path: self.targets.state_file().to_path_buf(),
                });
            }
            let history = self.journal_reader.events_through(batch.to()).await?;
            if history.scanned_to().unwrap_or(GlobalSeqNr::ZERO) != batch.to() {
                return Err(CatchUpError::PlanUnavailable);
            }
            let tables = ReadTables::project(&history)?;
            self.journal_reader
                .publish(&self.projection, &batch, &tables)
                .await?;
            // 保存済みの断面はここで確定した。追加イベントはその計画へ混ぜず、
            // 下の通常処理で別の計画として公開してから呼出元へ戻る。
        }
        self.catch_up_steering().await?;

        let checkpoint = self.journal_reader.checkpoint(&self.projection).await?;
        // 差分読取は「進む先があるか」の**探り**にだけ使う。ここで得た行を描く材料に
        // 使ってはいけない — 構造化面は全履歴を要するので読取が 2 回になり、その間に
        // 入った書込のぶんだけ 2 つの断面が食い違う (Markdown 面は古く、行は新しく、
        // `as_of` がチェックポイントを追い越す)。
        if self
            .journal_reader
            .events_after(checkpoint)
            .await?
            .scanned_to()
            .is_none()
        {
            return Ok(checkpoint);
        }

        // 描く材料はすべてこの**1 回の読取**から採る — Markdown 面の差分・構造化面の行・
        // 前進先の 3 つが同じ断面を指す。
        let history = self.journal_reader.events_after(GlobalSeqNr::ZERO).await?;
        let Some(last) = history.scanned_to() else {
            return Ok(checkpoint);
        };

        // 未投影の実行イベントがあるときだけ描く。intent の行しか無い区間は書くものが
        // 無い — それでもチェックポイントは走査済み位置まで進める（intent 行を毎回
        // 再走査しない。issue #56 申し送りの解消）。行は global 通番の昇順なので、
        // 境界は二分探索で 1 か所に定まる。
        let executions = history.executions();
        let unprojected = executions
            .split_at(executions.partition_point(|entry| entry.global_seq() <= checkpoint))
            .1;
        let mut files = Vec::new();
        if !unprojected.is_empty() {
            let plan = self.resolve_plan(&history)?;
            let state = crate::workspace::read_state_file(self.targets.state_file())
                .map_err(CatchUpError::StateFileRead)?;
            let before_state = state.clone();
            let mut read_model = ReadModel::new(state);
            // メモリ層は**在るとは限らない面**である（b49）。2 本とも在るときだけ載せる —
            // 片方だけ在るのは載せない（存在の検査の正本は動詞側にある）。
            let memory_before = self.read_memory_faces()?;
            if let Some((team, project)) = &memory_before {
                read_model = read_model.with_memory(team.clone(), project.clone());
            }
            crate::workspace::project(unprojected, &plan, &mut read_model)?;

            // 書く順は upstream の Step 5〜7 の写しである: project.md → team.md →
            // 状態ファイル → 監査シャード。project.md が先なのは、そちらの書込が失敗しても
            // team.md が無傷で残るからである（ピン `3c3146cf` `aidlc-state.ts:3705-3723`）。
            // メモリ層は**書き替えたときだけ**書く — 人が編集する正本でもあるので、
            // 触っていないキャッチアップが mtime を動かしてはならない。
            if let Some(memory) = read_model.memory() {
                let (team, project) = memory_before
                    .as_ref()
                    .ok_or(CatchUpError::PlanUnavailable)?;
                files.push(PublicationFile::memory(
                    self.targets.project_md(),
                    project,
                    memory.project(),
                ));
                files.push(PublicationFile::memory(
                    self.targets.team_md(),
                    team,
                    memory.team(),
                ));
            }
            files.push(PublicationFile::replacement(
                self.targets.state_file(),
                &before_state,
                read_model.state(),
            ));
            if !read_model.appended_audit().is_empty() {
                files.push(PublicationFile::audit(
                    self.targets.audit_shard(),
                    read_model.appended_audit(),
                )?);
            }
        }

        // 構造化面は差分投影ではなく全再計算である（同じ履歴から作る）。
        let tables = ReadTables::project(&history)?;

        let batch = PublicationBatch::new(checkpoint, last, files).for_targets(&self.targets)?;
        self.journal_reader
            .publish(&self.projection, &batch, &tables)
            .await?;
        Ok(last)
    }

    /// メモリ層 2 本の本文（**両方在るときだけ**）。
    ///
    /// 2 本が揃っていないのは正常である — 昇格を 1 度も打っていない workspace には
    /// そもそも投影する面が要らない。在るのに読めないのは blocking で、`MemoryFileRead`
    /// として止める（読めないまま進むと受領証だけが立って正本が古いままになる）。
    fn read_memory_faces(&self) -> Result<Option<(String, String)>, CatchUpError> {
        let team = self.targets.team_md();
        let project = self.targets.project_md();
        if !team.exists() || !project.exists() {
            return Ok(None);
        }
        Ok(Some((read_memory_file(team)?, read_memory_file(project)?)))
    }

    /// 参照入力が動いていれば steering の面を作り直す。
    ///
    /// 読むのは毎回である — 規則ファイルの編集はイベントを伴わないので、「動いたかどうか」を
    /// 読まずに知る手立てが無い。読んだうえで [`MemoryRules::source_digest`] を保存済みの値と
    /// 比べ、**同じなら 1 行も書かない**。毎回書き替えると、規則を 1 文字も触っていないのに
    /// 束のバイトが動きうる。
    ///
    /// [`MemoryRules::source_digest`]: crate::read_tables::MemoryRules::source_digest
    async fn catch_up_steering(&mut self) -> Result<(), CatchUpError> {
        let rules = self.steering.read()?;
        let source_digest = rules.source_digest();
        if self.journal_reader.steering_source_digest().await? == Some(source_digest) {
            return Ok(());
        }
        let tables = SteeringTables::pack(&rules)?;
        self.journal_reader.replace_steering(&tables).await?;
        Ok(())
    }

    /// 解決済み計画を得る（初回だけ履歴から引く）。
    ///
    /// 計画（表示属性・走査結果）の正本は intent 自身の誕生記録（`Created`）であり、どの
    /// intent かは実行の `Started` が指す（issue #56）。未投影の差分にその 2 行が入って
    /// いるとは限らないので、探すのは全履歴からである。取ってくるのは**この層の仕事**で
    /// ある — 投影核は計画を受け取るだけで、どこから来たかを知らない（二層構造）。
    ///
    /// どちらもワークフローごとに 1 度しか書かれないので、一度引けば控えを使い回す。
    ///
    /// 履歴は**呼出側が読んだものを受け取る** — ここで独自に読み直すと、キャッチアップ 1 回の
    /// 中で断面がもう 1 つ増えてしまう。
    fn resolve_plan(&mut self, history: &JournalBatch) -> Result<ResolvedPlan, CatchUpError> {
        if let Some(plan) = &self.plan {
            return Ok(plan.clone());
        }
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

/// メモリ層のファイルを 1 本読む（在るのに読めないのは blocking）。
fn read_memory_file(path: &std::path::Path) -> Result<String, CatchUpError> {
    std::fs::read_to_string(path).map_err(|error| CatchUpError::MemoryFileRead {
        path: path.display().to_string(),
        kind: error.kind(),
    })
}
