//! 計画指紋の参照面（`read_plan_fingerprint`）を最新化する更新器。
//!
//! 形は「ジャーナルを読む → 投影 (純粋な変換) → 表の DAO で書く」である
//! (`coding-rules/read-model-updater-structure.md`)。材料の片方は承認入力 (計画・手順書・規則)
//! なので、冪等の鍵は処理したシーケンス番号ではなく行の `source_digest` である。
//!
//! ```text
//! JournalReader.events_after(0) → 投影: PlanFingerprintTables::project
//!   BEGIN IMMEDIATE ─────────────────────────────────────────────── COMMIT
//!     fingerprint DAO.find_stamp ── 描き直す必要が無ければ何も書かない
//!     fingerprint DAO.save
//! ```

use std::path::{Path, PathBuf};

use core_command_domain::orchestration::{IntentExecutionId, PlanApprovalInput};
use rusqlite::{Connection, TransactionBehavior};

use super::global_seq_nr::GlobalSeqNr;
use super::journal_reader::JournalReader;
use super::read_model_update_error::ReadModelUpdateError;
use super::read_model_updater::ReadModelUpdater;
use super::store_failure::SqliteResultExt;
use super::{PlanFingerprintDao, PlanFingerprintDaoImpl, SourceStamp, updater_connection};

/// 計画指紋の参照面を、対象実行の履歴と現在の承認入力から再投影する。
///
/// 読み手・実行・承認入力は**借りる**。承認入力は呼出側が指紋の検証後も続けて使う
/// （計画・手順書の本文）ので、更新器が所有を奪わない。表を書く接続は更新器が所有し、
/// `BEGIN IMMEDIATE` で開いたトランザクションを表の DAO へ渡す。型引数 `F` は表の DAO で
/// ある (スタティックディスパッチ)。
#[derive(Debug)]
pub struct PlanFingerprintReadModelUpdater<'a, R, F> {
    journal_reader: &'a mut R,
    execution_id: &'a IntentExecutionId,
    input: &'a PlanApprovalInput,
    connection: Connection,
    path: PathBuf,
    fingerprints: F,
}

impl<'a, R: JournalReader> PlanFingerprintReadModelUpdater<'a, R, PlanFingerprintDaoImpl> {
    /// 読み手・対象実行・現在の承認入力を束ね、既存の共有ストアへ接続する。表が無ければ
    /// 作る。ストア自体は作らない。
    ///
    /// # Errors
    ///
    /// ストアへ接続できない、表を作れない場合。
    pub fn open(
        journal_reader: &'a mut R,
        path: &Path,
        execution_id: &'a IntentExecutionId,
        input: &'a PlanApprovalInput,
    ) -> Result<Self, ReadModelUpdateError> {
        let mut connection = updater_connection::open(path)?;
        let fingerprints = PlanFingerprintDaoImpl;
        let mut transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .at_store(path)?;
        fingerprints.create_table(&mut transaction)?;
        transaction.commit().at_store(path)?;
        Ok(Self {
            journal_reader,
            execution_id,
            input,
            connection,
            path: path.to_path_buf(),
            fingerprints,
        })
    }
}

impl<R: JournalReader, F: PlanFingerprintDao> ReadModelUpdater
    for PlanFingerprintReadModelUpdater<'_, R, F>
{
    type Error = ReadModelUpdateError;

    /// 全履歴から計画指紋の行を組み、描き直す必要があれば差し替える。
    ///
    /// 保存済みの行を `BEGIN IMMEDIATE` で書込ロックを取ってから読み、同じ照合子 (同じ入力から
    /// 作られた行) か、より新しい履歴位置の行なら書かない。IMMEDIATE で開くのは、読んでから
    /// 書くトランザクションを DEFERRED で始めると、別の書き手がいるときの書込昇格が
    /// busy timeout を待たずに即失敗するからである (#134)。
    ///
    /// # Errors
    ///
    /// 履歴の再構成または参照面の読み書きに失敗した場合。
    async fn update_read_models(&mut self) -> Result<(), ReadModelUpdateError> {
        self.journal_reader.prepare_read_model()?;
        let history = self.journal_reader.events_after(GlobalSeqNr::ZERO).await?;
        let tables = crate::read_tables::PlanFingerprintTables::project(
            &history,
            self.execution_id,
            self.input,
        )?;
        let row = tables.row();
        let mut transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .at_store(&self.path)?;
        let stamp = self.fingerprints.find_stamp(&transaction, row.id())?;
        if already_projected(stamp.as_ref(), row.source_digest(), row.as_of()) {
            return Ok(());
        }
        self.fingerprints.save(&mut transaction, row)?;
        transaction.commit().at_store(&self.path)?;
        Ok(())
    }
}

/// 保存済みの行が、いま組んだ行を書く必要を無くしているか。
///
/// 同じ入力から作られている (照合子が同じ) か、より新しい履歴位置で作られている
/// (古い断面で新しい面を上書きしない) なら書かない。
fn already_projected(stamp: Option<&SourceStamp>, source_digest: &str, as_of: GlobalSeqNr) -> bool {
    stamp.is_some_and(|stamp| stamp.source_digest() == source_digest || stamp.as_of() > as_of)
}
