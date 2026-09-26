//! テスト契約の参照面（`read_testing_contract`）を最新化する更新器。
//!
//! 形は「ジャーナルを読む → 投影 (純粋な変換) → 表の DAO で書く」である
//! (`coding-rules/read-model-updater-structure.md`)。材料の片方は人が編集する規則ファイルなので、
//! 冪等の鍵は処理したシーケンス番号ではなく行の `source_digest` である。
//!
//! ```text
//! JournalReader.events_after(0) + 規則の読取 → 投影: TestingTables::project
//!   contract DAO.find_stamp (ロックを取らずに比較) ── 描き直す必要が無ければ何も書かない
//!   BEGIN IMMEDIATE ─────────────────────────────────────────────── COMMIT
//!     contract DAO.find_stamp (比べ直す) ── 描き直す必要が無ければ何も書かない
//!     contract DAO.replace
//! ```

use std::path::{Path, PathBuf};

use rusqlite::{Connection, TransactionBehavior};

use super::global_seq_nr::GlobalSeqNr;
use super::journal_reader::JournalReader;
use super::read_model_update_error::ReadModelUpdateError;
use super::read_model_updater::ReadModelUpdater;
use super::steering_source::SteeringSource;
use super::store_failure::SqliteResultExt;
use super::{SourceStamp, TestingContractDao, TestingContractDaoImpl, updater_connection};

/// 出所を照合する行 — 依頼前の既定行。どの断面にも必ず在り、全行が同じ出所を名乗る。
const BARE_SPACE: &str = "bare-space";

/// テスト契約の参照面を、全履歴と memory 層の規則から描き直す。
///
/// 純粋な表示や承認の入力確認の前に呼ぶ。材料の片方は人が編集する規則ファイルなので、
/// 描き直すのは規則と依頼条件の照合子 (`source_digest`) が保存済みの値から動いたときだけで
/// ある。保存済みの行がより新しい履歴位置 (`as_of`) で作られていれば、古い断面で上書きしない。
///
/// 読み手と規則の読取先は**借りる**。呼出側が 1 回の更新のために開いたものを、更新器が
/// 所有する理由が無いため。表を書く接続は更新器が所有し、`BEGIN IMMEDIATE` で開いた
/// トランザクションを表の DAO へ渡す。型引数 `T` は表の DAO である (スタティックディスパッチ)。
///
/// 共有構造化面の点検 (古ければ描き直す) はしない — それは構造化面の更新器
/// ([`super::StructuredReadModelUpdater`]) の仕事で、呼出側がこの更新器より先に起動する
/// (Issue #153 の PR4 までは、借りた読み手の `prepare_read_model` をここで呼んでいた)。
#[derive(Debug)]
pub struct TestingReadModelUpdater<'a, R, T> {
    journal_reader: &'a R,
    source: &'a SteeringSource,
    connection: Connection,
    path: PathBuf,
    contracts: T,
}

impl<'a, R: JournalReader> TestingReadModelUpdater<'a, R, TestingContractDaoImpl> {
    /// 読み手と、テスト契約の規則を読む先を束ね、既存の共有ストアへ接続する。表が無ければ
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
        source: &'a SteeringSource,
    ) -> Result<Self, ReadModelUpdateError> {
        let mut connection = updater_connection::open(path)?;
        let contracts = TestingContractDaoImpl;
        if !contracts.table_exists(&connection)? {
            let mut transaction = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .at_store(path)?;
            contracts.create_table(&mut transaction)?;
            transaction.commit().at_store(path)?;
        }
        Ok(Self {
            journal_reader,
            source,
            connection,
            path: path.to_path_buf(),
            contracts,
        })
    }
}

impl<R: JournalReader, T: TestingContractDao> ReadModelUpdater
    for TestingReadModelUpdater<'_, R, T>
{
    type Error = ReadModelUpdateError;

    /// 全履歴と規則からテスト契約の行を組み、描き直す必要があれば差し替える。
    ///
    /// 最初の比較は書込ロックを取らずに行う (規則を触っていない更新はロックを取り合わない)。
    /// 描き直すときは `BEGIN IMMEDIATE` で書込ロックを取ってから比べ直す。IMMEDIATE で開くのは、
    /// 読んでから書くトランザクションを DEFERRED で始めると、別の書き手がいるときの書込昇格が
    /// busy timeout を待たずに即失敗するからである (#134)。
    ///
    /// # Errors
    ///
    /// 履歴、規則、参照面の読書きに失敗した場合。
    async fn update_read_models(&mut self) -> Result<(), ReadModelUpdateError> {
        let history = self.journal_reader.events_after(GlobalSeqNr::ZERO).await?;
        let sections = self.source.read_testing_sections()?;
        let tables = crate::read_tables::TestingTables::project(&history, &sections);
        let current = |stamp: Option<SourceStamp>| {
            already_projected(stamp.as_ref(), tables.source_digest(), tables.as_of())
        };
        if current(self.contracts.find_stamp(&self.connection, BARE_SPACE)?) {
            return Ok(());
        }
        let mut transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .at_store(&self.path)?;
        if current(self.contracts.find_stamp(&transaction, BARE_SPACE)?) {
            return Ok(());
        }
        self.contracts.replace(
            &mut transaction,
            tables.rows(),
            tables.source_digest(),
            tables.as_of(),
        )?;
        transaction.commit().at_store(&self.path)?;
        Ok(())
    }
}

/// 保存済みの行が、いま組んだ行を書く必要を無くしているか。
///
/// 同じ参照入力から作られている (照合子が同じ) か、より新しい履歴位置で作られている
/// (古い断面で新しい面を上書きしない) なら書かない。
fn already_projected(stamp: Option<&SourceStamp>, source_digest: &str, as_of: GlobalSeqNr) -> bool {
    stamp.is_some_and(|stamp| stamp.source_digest() == source_digest || stamp.as_of() > as_of)
}
