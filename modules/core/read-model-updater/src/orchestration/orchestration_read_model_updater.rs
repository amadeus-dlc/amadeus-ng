//! **取得ループ** — RMU の上側の層（2026-08-28 裁定 / `coding-rules/cqrs-boundaries.md`）。
//!
//! ```text
//! steering の更新器 (参照入力の読取 → source_digest 比較 → 変化時のみ 2 表を DAO で差し替え) ← 別 Tx
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
//!
//! 更新の入口は共通契約 [`ReadModelUpdater`] の `update_read_models` である。構造化面だけ・
//! テスト契約・計画指紋・Code Generation 開始可否を描く入口は、それぞれ別の更新器
//! （[`super::StructuredReadModelUpdater`] ほか）に分けてある。

use core_command_domain::orchestration::IntentExecutionEvent;
use core_command_domain::workspace::EventType;

use crate::read_tables::ReadTables;
use crate::workspace::{ReadModel, ResolvedPlan};

use super::global_seq_nr::GlobalSeqNr;
use super::journal_batch::JournalBatch;
use super::journal_reader::JournalReader;
use super::projection_name::ProjectionName;
use super::projection_targets::ProjectionTargets;
use super::read_model_update_error::ReadModelUpdateError;
use super::read_model_updater::ReadModelUpdater;
use super::{PublicationBatch, PublicationFile};

/// 取得ループ — チェックポイント以降のイベントを Markdown 面と構造化面の両方へ流し込む。
///
/// 名前に `Orchestration` を冠するのは、orchestration コンテキストの本体（状態ファイル・
/// 監査シャード・`read_*` 表を 1 回で描く更新器）であることを、同じ契約を実装する他の
/// 更新器（構造化面だけ・runtime-graph・心拍 …）と区別するためである。
///
/// 型引数 `S` は steering の面を描く更新器である（実物は [`super::SteeringReadModelUpdater`]）。
/// steering の面は参照入力由来で、自分の表の DAO と自分のトランザクションで書く。取得ループは
/// その更新を**どの時点で起動するか**（ジャーナル差分の探りより前）だけを持つ。
#[derive(Debug)]
pub struct OrchestrationReadModelUpdater<R, S> {
    pipeline_handoff: Option<Option<core_command_domain::orchestration::PipelineHandoff>>,
    journal_reader: R,
    projection: ProjectionName,
    targets: ProjectionTargets,
    /// 参照入力 (memory 層) から steering の面を描く更新器。ジャーナルとは別の入口である。
    steering: S,
    /// 解決済み計画の控え。`Started` は 1 度しか書かれないので、一度引けば以後は使い回す。
    plan: Option<ResolvedPlan>,
    execution_id: Option<core_command_domain::orchestration::IntentExecutionId>,
}

impl<R: JournalReader, S> OrchestrationReadModelUpdater<R, S> {
    /// 現在のhandoffを参照入力として受け取り、イベントなしの変更も再投影する。
    #[must_use]
    pub fn with_pipeline_handoff(
        mut self,
        handoff: Option<core_command_domain::orchestration::PipelineHandoff>,
    ) -> Self {
        self.pipeline_handoff = Some(handoff);
        self
    }
    async fn update_pipeline(&mut self) -> Result<(), ReadModelUpdateError> {
        if let (Some(current), Some(id)) = (&self.pipeline_handoff, &self.execution_id) {
            let history = self.journal_reader.events_after(GlobalSeqNr::ZERO).await?;
            let tables =
                crate::read_tables::PipelineTables::project(&history, id, current.as_ref())?;
            self.journal_reader.replace_pipeline(&tables).await?;
        }
        Ok(())
    }
    /// 旧共有投影が既に公開されていれば、別名での再生を止める。
    ///
    /// 更新ではなく**更新前の関門**なので [`ReadModelUpdater`] の契約には載せない。守るのは
    /// 本更新器が描く実行別の投影名（旧共有投影 → 実行別への移行）であり、読み手を組み込む
    /// 前に呼ぶので、インスタンスではなく読み手を受け取る関連関数にしてある。
    ///
    /// # Errors
    /// チェックポイントを読めない場合、または旧投影が公開済みの場合。
    pub async fn require_unpublished(
        journal_reader: &R,
        legacy_projection: &ProjectionName,
    ) -> Result<(), ReadModelUpdateError> {
        if journal_reader.checkpoint(legacy_projection).await? != GlobalSeqNr::ZERO {
            return Err(ReadModelUpdateError::LegacyProjection {
                projection: legacy_projection.as_str().to_string(),
            });
        }
        Ok(())
    }

    /// 読み手・投影名・書込先・steering の面の更新器から組む。
    pub const fn new(
        journal_reader: R,
        projection: ProjectionName,
        targets: ProjectionTargets,
        steering: S,
    ) -> OrchestrationReadModelUpdater<R, S> {
        OrchestrationReadModelUpdater {
            pipeline_handoff: None,
            journal_reader,
            projection,
            targets,
            steering,
            plan: None,
            execution_id: None,
        }
    }

    /// 状態・監査へ反映する実行を明示する。構造化面は引き続き全履歴から投影する。
    #[must_use]
    pub fn for_execution(
        mut self,
        id: core_command_domain::orchestration::IntentExecutionId,
    ) -> Self {
        self.execution_id = Some(id);
        self.plan = None;
        self
    }

    /// 書込先の場所。
    #[must_use]
    pub const fn targets(&self) -> &ProjectionTargets {
        &self.targets
    }

    /// 本更新器の投影がどこまで描き終えたか（投影チェックポイント）。
    ///
    /// 更新（[`ReadModelUpdater::update_read_models`]）はコマンドなので前進先を返さない。
    /// 到達点を知りたい側はこのクエリで読む（`coding-rules/command-query-separation.md` の
    /// 「分離できるなら 2 つのメソッドに分離する」）。差分が空だった更新の後は、更新前と
    /// 同じ値が返る。
    ///
    /// # Errors
    ///
    /// チェックポイントを読めない場合。
    pub async fn checkpoint(&self) -> Result<GlobalSeqNr, ReadModelUpdateError> {
        Ok(self.journal_reader.checkpoint(&self.projection).await?)
    }
}

impl<R, S> ReadModelUpdater for OrchestrationReadModelUpdater<R, S>
where
    R: JournalReader,
    S: ReadModelUpdater<Error = ReadModelUpdateError>,
{
    type Error = ReadModelUpdateError;

    /// チェックポイント以降を読んで描き、チェックポイントを進める。
    ///
    /// 差分が空なら**何も書かない**（チェックポイントも動かない）。前進先は
    /// [`OrchestrationReadModelUpdater::checkpoint`] で読む。
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
    /// 1 回の更新で Markdown 面（系統 (1) — `aidlc-state.md` と監査シャード）と
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
    /// 空でも参照入力は見る — したがって差分の探りより**前**に steering の更新器を起動する。
    /// 読むのは毎回だが、書き替えるのは `source_digest` が動いたときだけであり、その比較と
    /// 差し替えはチェックポイントとは別のトランザクションである（設計 §3 —
    /// [`super::SteeringReadModelUpdater`]）。
    ///
    /// # Errors
    ///
    /// ジャーナルの読取・チェックポイントの失敗（`Read`）、投影核が描けなかった
    /// （`Projection`）、状態ファイルを読めない（`StateFileRead`）・書けない
    /// （`StateFileWrite`）、公開先を読めない・追記できない（`PublicationIo`）、構造化投影核が
    /// 歴史の切り落としを見つけた（`ReadTables`）、参照入力の規則ファイルが在るのに読めない
    /// （`SteeringRead`）・刻めない（`SteeringPack`）。
    async fn update_read_models(&mut self) -> Result<(), ReadModelUpdateError> {
        self.journal_reader.prepare_read_model()?;
        if let Some(batch) = self
            .journal_reader
            .pending_publication(&self.projection)
            .await?
        {
            if !batch.matches_targets(&self.targets) {
                return Err(ReadModelUpdateError::PublicationConflict {
                    path: self.targets.state_file().to_path_buf(),
                });
            }
            let history = self.journal_reader.events_through(batch.to()).await?;
            if history.scanned_to().unwrap_or(GlobalSeqNr::ZERO) != batch.to() {
                return Err(ReadModelUpdateError::PlanUnavailable);
            }
            let tables = ReadTables::project(&history)?;
            self.journal_reader
                .publish(&self.projection, &batch, &tables)
                .await?;
            // 保存済みの断面はここで確定した。追加イベントはその計画へ混ぜず、
            // 下の通常処理で別の計画として公開してから呼出元へ戻る。
        }
        self.steering.update_read_models().await?;
        self.update_pipeline().await?;

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
            return Ok(());
        }

        // 描く材料はすべてこの**1 回の読取**から採る — Markdown 面の差分・構造化面の行・
        // 前進先の 3 つが同じ断面を指す。
        let history = self.journal_reader.events_after(GlobalSeqNr::ZERO).await?;
        let last = history
            .scanned_to()
            .ok_or(ReadModelUpdateError::HistoryDisappeared)?;

        // 未投影の実行イベントがあるときだけ描く。intent の行しか無い区間は書くものが
        // 無い — それでもチェックポイントは走査済み位置まで進める（intent 行を毎回
        // 再走査しない。issue #56 申し送りの解消）。行は global 通番の昇順なので、
        // 境界は二分探索で 1 か所に定まる。
        let executions: Vec<_> = history
            .executions()
            .iter()
            .filter(|entry| {
                self.execution_id
                    .as_ref()
                    .is_none_or(|id| entry.execution_id() == id)
            })
            .cloned()
            .collect();
        let unprojected = executions
            .split_at(executions.partition_point(|entry| entry.global_seq() <= checkpoint))
            .1;
        let mut files = Vec::new();
        // 監査シャードへ追記する値は、投影の行もフックの直接行も同じ規則で project dir を
        // 伏せる (upstream `renderAuditBlock` — 描画の出口で 1 度だけ掛ける)。
        let redaction = self.targets.audit_redaction();
        if let Some(prompt) = unprojected
            .iter()
            .rev()
            .find(|entry| matches!(entry.event(), IntentExecutionEvent::PromptObserved(_)))
        {
            let path = self.targets.human_turn_file();
            let after = prompt
                .occurred_at()
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
                + "\n";
            let file = match std::fs::read_to_string(path) {
                Ok(before) => PublicationFile::replacement(path, &before, &after),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    PublicationFile::creation(path, &after)
                }
                Err(error) => {
                    return Err(ReadModelUpdateError::PublicationIo {
                        path: path.to_path_buf(),
                        kind: error.kind(),
                    });
                }
            };
            files.push(file);
        }
        for entry in unprojected {
            let baseline = match entry.event() {
                IntentExecutionEvent::Jumped(jump) => jump.baseline(),
                IntentExecutionEvent::Reported(report) => report.source_baseline(),
                _ => None,
            };
            if let Some(baseline) = baseline
                && let (Some(name), Some(listing)) = (baseline.snapshot_name(), baseline.listing())
            {
                files.push(PublicationFile::creation(
                    &self.targets.source_baseline_file(&name),
                    listing,
                ));
            }
        }
        if !unprojected.is_empty() {
            let plan = self.resolve_plan(&history)?;
            let genesis = unprojected.first().and_then(|entry| match entry.event() {
                IntentExecutionEvent::Started(started) => Some((entry, started)),
                _ => None,
            });
            let missing_state = !self.targets.state_file().try_exists().map_err(|error| {
                ReadModelUpdateError::StateFileRead(crate::workspace::StateFileReadError::new(
                    error.to_string(),
                ))
            })?;
            let before_state = if missing_state && genesis.is_some() {
                String::new()
            } else {
                crate::workspace::read_state_file(self.targets.state_file())
                    .map_err(ReadModelUpdateError::StateFileRead)?
            };
            let mut state = before_state.clone();
            if let Some((entry, started)) = genesis {
                let intent = history
                    .intents()
                    .iter()
                    .find(|intent| intent.id() == started.intent_id())
                    .ok_or(ReadModelUpdateError::PlanUnavailable)?;
                if let Some(baseline) = intent.source_baseline()
                    && let (Some(name), Some(listing)) =
                        (baseline.snapshot_name(), baseline.listing())
                {
                    files.push(PublicationFile::creation(
                        &self.targets.source_baseline_file(&name),
                        listing,
                    ));
                }
                if missing_state {
                    state = crate::workspace::initial_state::compose(
                        intent,
                        ".",
                        &entry
                            .occurred_at()
                            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                    );
                }
                let description = core_infrastructure::canon_json::serialize(
                    &core_infrastructure::canon_json::JsonValue::String(
                        intent.request().to_string(),
                    ),
                    core_infrastructure::canon_json::SerializationProfile::ContractCompact,
                ) + "\n";
                if !self
                    .targets
                    .description_file()
                    .try_exists()
                    .map_err(|error| {
                        ReadModelUpdateError::StateFileRead(
                            crate::workspace::StateFileReadError::new(error.to_string()),
                        )
                    })?
                {
                    files.push(PublicationFile::creation(
                        self.targets.description_file(),
                        &description,
                    ));
                }
            }
            let mut read_model = ReadModel::new(state);
            // メモリ層は**在るとは限らない面**である（b49）。2 本とも在るときだけ載せる —
            // 片方だけ在るのは載せない（存在の検査の正本は動詞側にある）。
            let memory_before = self.read_memory_faces()?;
            if let Some((team, project)) = &memory_before {
                read_model = read_model.with_memory(team.clone(), project.clone());
            }
            crate::workspace::project(unprojected, &plan, &mut read_model)?;
            if let Some(after) = read_model.active_directive() {
                let path = self.targets.active_directive_file();
                let file = match std::fs::read_to_string(path) {
                    Ok(before) => PublicationFile::replacement(path, &before, after),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        PublicationFile::creation(path, after)
                    }
                    Err(error) => {
                        return Err(ReadModelUpdateError::PublicationIo {
                            path: path.to_path_buf(),
                            kind: error.kind(),
                        });
                    }
                };
                files.push(file);
            }

            // 書く順は upstream の Step 5〜7 の写しである: project.md → team.md →
            // 状態ファイル → 監査シャード。project.md が先なのは、そちらの書込が失敗しても
            // team.md が無傷で残るからである（ピン `3c3146cf` `aidlc-state.ts:3705-3723`）。
            // メモリ層は**書き替えたときだけ**書く — 人が編集する正本でもあるので、
            // 触っていない更新が mtime を動かしてはならない。
            if let (Some((team, project)), Some(memory)) =
                (memory_before.as_ref(), read_model.memory())
            {
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
            files.push(if missing_state {
                PublicationFile::creation(self.targets.state_file(), read_model.state())
            } else {
                PublicationFile::replacement(
                    self.targets.state_file(),
                    &before_state,
                    read_model.state(),
                )
            });
            if !read_model.appended_audit().is_empty() {
                files.push(PublicationFile::audit(
                    self.targets.audit_shard(),
                    &redaction.redact(read_model.appended_audit()),
                )?);
            }
        }

        // ArtifactSavedも同じ履歴断面から同じPublicationBatchへ積む。
        if let Some(target) = self.targets.audit_target() {
            let mut blocks = String::new();
            for entry in history
                .artifacts()
                .iter()
                .filter(|entry| entry.global_seq() > checkpoint)
            {
                let observation = entry.event().observation();
                if observation.target().relative_directory() != target {
                    continue;
                }
                let kind = if observation.created() {
                    EventType::ArtifactCreated
                } else {
                    EventType::ArtifactUpdated
                };
                let mut fields = core_command_domain::workspace::AuditFields::new();
                for (key, value) in [
                    ("Tool", observation.tool()),
                    ("File", observation.file()),
                    ("Context", observation.context()),
                ] {
                    let key = core_command_domain::workspace::AuditFieldKey::parse(key)
                        .map_err(|_| ReadModelUpdateError::PlanUnavailable)?;
                    fields = fields.with(key, value);
                }
                blocks.push_str(&crate::workspace::render_audit_block(
                    kind,
                    entry.occurred_at(),
                    &fields,
                ));
            }
            if !blocks.is_empty() {
                files.push(PublicationFile::audit(
                    self.targets.audit_shard(),
                    &redaction.redact(&blocks),
                )?);
            }
        }

        if let Some(target) = self.targets.audit_target() {
            let mut blocks = String::new();
            for entry in history
                .sessions()
                .iter()
                .filter(|entry| entry.global_seq() > checkpoint)
            {
                let event = entry.event();
                if event.target().relative_directory() == target {
                    blocks.push_str(&crate::workspace::render_audit_block(
                        event.record().kind(),
                        entry.occurred_at(),
                        event.record().fields(),
                    ));
                }
            }
            if !blocks.is_empty() {
                files.push(PublicationFile::audit(
                    self.targets.audit_shard(),
                    &redaction.redact(&blocks),
                )?);
            }
        }

        // 構造化面は差分投影ではなく全再計算である（同じ履歴から作る）。
        let audit_only = (!history.artifacts().is_empty() || !history.sessions().is_empty())
            && history.definitions().is_empty()
            && history.intents().is_empty()
            && history.executions().is_empty();
        let tables = if audit_only {
            ReadTables::project_audit_only(&history)?
        } else {
            ReadTables::project(&history)?
        };
        if let Some(registry) =
            super::intent_registry::publication(&history, &tables, self.targets.registry_file())?
        {
            files.push(registry);
        }

        let batch = PublicationBatch::new(checkpoint, last, files).for_targets(&self.targets)?;
        self.journal_reader
            .publish(&self.projection, &batch, &tables)
            .await?;
        Ok(())
    }
}

impl<R: JournalReader, S> OrchestrationReadModelUpdater<R, S> {
    /// メモリ層 2 本の本文（**両方在るときだけ**）。
    ///
    /// 2 本が揃っていないのは正常である — 昇格を 1 度も打っていない workspace には
    /// そもそも投影する面が要らない。在るのに読めないのは blocking で、`MemoryFileRead`
    /// として止める（読めないまま進むと受領証だけが立って正本が古いままになる）。
    fn read_memory_faces(&self) -> Result<Option<(String, String)>, ReadModelUpdateError> {
        let team = self.targets.team_md();
        let project = self.targets.project_md();
        if !team.exists() || !project.exists() {
            return Ok(None);
        }
        Ok(Some((read_memory_file(team)?, read_memory_file(project)?)))
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
    /// 履歴は**呼出側が読んだものを受け取る** — ここで独自に読み直すと、更新 1 回の
    /// 中で断面がもう 1 つ増えてしまう。
    fn resolve_plan(
        &mut self,
        history: &JournalBatch,
    ) -> Result<ResolvedPlan, ReadModelUpdateError> {
        if let Some(plan) = &self.plan {
            return Ok(plan.clone());
        }
        let mut started_ids = history
            .executions()
            .iter()
            .filter(|entry| {
                self.execution_id
                    .as_ref()
                    .is_none_or(|id| entry.execution_id() == id)
            })
            .filter_map(|entry| match entry.event() {
                IntentExecutionEvent::Started(started) => Some(started.intent_id().clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        started_ids.dedup();
        // 単一 intent が本ループの契約である — 混在を黙って 1 つの計画で描かない
        // (CodeRabbit 指摘。intent ごとの振り分けは U7 の駆動設計と対で扱う)。
        if started_ids.len() > 1 {
            return Err(ReadModelUpdateError::MixedIntents);
        }
        let plan = started_ids
            .first()
            .and_then(|id| history.intents().iter().find(|intent| intent.id() == id))
            .map(ResolvedPlan::of)
            .ok_or(ReadModelUpdateError::PlanUnavailable)?;
        self.plan = Some(plan.clone());
        Ok(plan)
    }
}

/// メモリ層のファイルを 1 本読む（在るのに読めないのは blocking）。
fn read_memory_file(path: &std::path::Path) -> Result<String, ReadModelUpdateError> {
    std::fs::read_to_string(path).map_err(|error| ReadModelUpdateError::MemoryFileRead {
        path: path.display().to_string(),
        kind: error.kind(),
    })
}
