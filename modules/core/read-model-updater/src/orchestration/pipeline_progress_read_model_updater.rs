//! Pipeline の参照面 (`read_pipeline_progress`) を最新化する更新器。
//!
//! 形は「ジャーナルを読む → 投影 (純粋な変換) → 表の DAO で書く」である
//! (`coding-rules/read-model-updater-structure.md`)。材料の片方は外部の handoff ファイルの観測
//! なので、冪等の鍵は処理したシーケンス番号ではなく行の `source_digest` である。
//!
//! ```text
//! JournalReader.events_after(0) → 投影: PipelineTables::project
//!   BEGIN IMMEDIATE ─────────────────────────────────────────────── COMMIT
//!     progress DAO.find_stamp ── 描き直す必要が無ければ何も書かない
//!     progress DAO.replace_for_execution
//! ```

use std::path::{Path, PathBuf};

use core_command_domain::orchestration::{IntentExecutionId, PipelineHandoff};
use rusqlite::{Connection, TransactionBehavior};

use crate::read_tables::PipelineTables;

use super::global_seq_nr::GlobalSeqNr;
use super::journal_reader::JournalReader;
use super::read_model_update_error::ReadModelUpdateError;
use super::read_model_updater::ReadModelUpdater;
use super::store_failure::SqliteResultExt;
use super::{PipelineProgressDao, PipelineProgressDaoImpl, SourceStamp, updater_connection};

/// Pipeline の参照面を、対象実行の全履歴と現在の handoff から再投影する。
///
/// 読み手・実行・handoff は**借りる**。取得ループ ([`super::OrchestrationReadModelUpdater`]) が
/// 1 回の更新のために持っているものを、更新器が所有する理由が無いため。表を書く接続は
/// 更新器が所有し、`BEGIN IMMEDIATE` で開いたトランザクションを表の DAO へ渡す。型引数 `P` は
/// 表の DAO である (スタティックディスパッチ)。
///
/// # 取得ループのチェックポイントとは束ねない
///
/// handoff ファイルはジャーナルの走査位置と無関係に変わる (イベントを伴わない) ので、
/// チェックポイントの前進と同じトランザクションに閉じる理由が無い。
#[derive(Debug)]
pub struct PipelineProgressReadModelUpdater<'a, R, P> {
    journal_reader: &'a R,
    execution_id: &'a IntentExecutionId,
    handoff: Option<&'a PipelineHandoff>,
    connection: Connection,
    path: PathBuf,
    progress: P,
}

impl<'a, R: JournalReader> PipelineProgressReadModelUpdater<'a, R, PipelineProgressDaoImpl> {
    /// 読み手・対象実行・現在の handoff を束ね、既存の共有ストアへ接続する。表が無ければ
    /// 作る。ストア自体は作らない。
    ///
    /// 表が在るか (`table_exists`) は書込ロックを取らずに見る。在れば書込トランザクションを
    /// 開かない — 別の書き手がいても開くだけで待たされない。無いときだけ `BEGIN IMMEDIATE` で
    /// 開いて作る (DEFERRED で開くと、スキーマを読んでから書込へ昇格するときに busy timeout を
    /// 待たずに即失敗しうる — #134)。
    ///
    /// # Errors
    ///
    /// ストアへ接続できない、表を作れない場合。
    pub fn open(
        journal_reader: &'a R,
        path: &Path,
        execution_id: &'a IntentExecutionId,
        handoff: Option<&'a PipelineHandoff>,
    ) -> Result<Self, ReadModelUpdateError> {
        let mut connection = updater_connection::open(path)?;
        let progress = PipelineProgressDaoImpl;
        if !progress.table_exists(&connection)? {
            let mut transaction = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .at_store(path)?;
            progress.create_table(&mut transaction)?;
            transaction.commit().at_store(path)?;
        }
        Ok(Self {
            journal_reader,
            execution_id,
            handoff,
            connection,
            path: path.to_path_buf(),
            progress,
        })
    }
}

impl<R: JournalReader, P: PipelineProgressDao> ReadModelUpdater
    for PipelineProgressReadModelUpdater<'_, R, P>
{
    type Error = ReadModelUpdateError;

    /// 全履歴と現在の handoff から対象実行の行を組み、描き直す必要があれば差し替える。
    ///
    /// 保存済みの出所を `BEGIN IMMEDIATE` で書込ロックを取ってから読み、同じ照合子 (同じ入力から
    /// 作られた行) か、より新しい履歴位置の行なら書かない。IMMEDIATE で開くのは、読んでから
    /// 書くトランザクションを DEFERRED で始めると、別の書き手がいるときの書込昇格が
    /// busy timeout を待たずに即失敗するからである (#134 — `aidlc-log link` が記録済みなのに
    /// 失敗を返した不具合の原因)。
    ///
    /// 組んだ行が 1 本も無い (実行や定義が履歴に無い、Pipeline のステージが無い) ときは比べる
    /// 相手が無いので、対象実行の行を消すだけの差し替えになる。
    ///
    /// # Errors
    ///
    /// 履歴の読取・再生、参照面の読み書き、トランザクションの確定に失敗した場合。
    async fn update_read_models(&mut self) -> Result<(), ReadModelUpdateError> {
        let history = self.journal_reader.events_after(GlobalSeqNr::ZERO).await?;
        let tables = PipelineTables::project(&history, self.execution_id, self.handoff)?;
        let mut transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .at_store(&self.path)?;
        let stamp = self
            .progress
            .find_stamp(&transaction, tables.execution_id())?;
        if let Some(first) = tables.rows().first()
            && already_projected(
                stamp.as_ref(),
                first.source_digest(),
                first.event_position(),
            )
        {
            return Ok(());
        }
        self.progress.replace_for_execution(
            &mut transaction,
            tables.execution_id(),
            tables.rows(),
        )?;
        transaction.commit().at_store(&self.path)?;
        Ok(())
    }
}

/// 保存済みの行が、いま組んだ行を書く必要を無くしているか。
///
/// 同じ入力から作られている (照合子が同じ) か、より新しい履歴位置で作られている
/// (古い断面で新しい面を上書きしない) なら書かない。
fn already_projected(
    stamp: Option<&SourceStamp>,
    source_digest: &str,
    event_position: GlobalSeqNr,
) -> bool {
    stamp.is_some_and(|stamp| {
        stamp.source_digest() == source_digest || stamp.as_of() > event_position
    })
}
