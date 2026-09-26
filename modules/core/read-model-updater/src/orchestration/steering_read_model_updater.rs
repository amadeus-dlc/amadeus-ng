//! steering の参照面 (`read_steering_plan` / `read_steering_part`) を最新化する更新器。
//!
//! 形は「参照入力を読む → 投影 (純粋な分割とパック) → 表の DAO で書く」である
//! (`coding-rules/read-model-updater-structure.md`)。材料はジャーナルではなく人が編集する
//! 規則ファイルなので、冪等の鍵は処理したシーケンス番号ではなく行の `source_digest` である。
//!
//! ```text
//! 参照入力の読取 (SteeringSource) → source_digest
//!   plan DAO.find_source_digest (ロックを取らずに比較) ── 同じなら何も書かない
//!   投影: SteeringTables::pack
//!   BEGIN IMMEDIATE ─────────────────────────────────────────────── COMMIT
//!     plan DAO.find_source_digest (比べ直す) ── 同じなら何も書かない
//!     plan DAO.replace / part DAO.replace     (表ごとに 1 本)
//! ```

use std::path::{Path, PathBuf};

use rusqlite::{Connection, TransactionBehavior};

use crate::read_tables::SteeringTables;

use super::store_failure::SqliteResultExt;
use super::{
    ReadModelUpdateError, ReadModelUpdater, SteeringPartDao, SteeringPartDaoImpl, SteeringPlanDao,
    SteeringPlanDaoImpl, SteeringSource, updater_connection,
};

/// steering の参照面の更新器。
///
/// 型引数は書く表ごとの DAO である (スタティックディスパッチ)。実物の組は
/// [`SteeringReadModelUpdater::open`] が作る。
///
/// # 2 表を 1 つのトランザクションで差し替える
///
/// 計画 (`read_steering_plan`) と部 (`read_steering_part`) は対でしか意味を持たない。
/// 更新器が `BEGIN IMMEDIATE` で開いたトランザクションを両表の DAO へ渡し、両方を書いてから
/// 確定する。途中で失敗すればトランザクションは確定されずに捨てられ、どちらの表も動かない。
///
/// # 取得ループのチェックポイントとは束ねない
///
/// 参照入力はジャーナルの走査位置と無関係に変わるので、チェックポイントの前進と同じ
/// トランザクションに閉じる理由が無い。束ねると規則を 1 文字直すたびにチェックポイントの
/// 書込ロックを取り合うことになる。
#[derive(Debug)]
pub struct SteeringReadModelUpdater<P, Q> {
    connection: Connection,
    path: PathBuf,
    source: SteeringSource,
    plans: P,
    parts: Q,
}

impl SteeringReadModelUpdater<SteeringPlanDaoImpl, SteeringPartDaoImpl> {
    /// 既存の共有ストアへ接続し、2 表が無ければ作る。ストア自体は作らない。
    ///
    /// # Errors
    ///
    /// ストアへ接続できない、表を作れない場合。
    pub fn open(path: &Path, source: SteeringSource) -> Result<Self, ReadModelUpdateError> {
        let mut connection = updater_connection::open(path)?;
        let plans = SteeringPlanDaoImpl;
        let parts = SteeringPartDaoImpl;
        let mut transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .at_store(path)?;
        plans.create_table(&mut transaction)?;
        parts.create_table(&mut transaction)?;
        transaction.commit().at_store(path)?;
        Ok(Self {
            connection,
            path: path.to_path_buf(),
            source,
            plans,
            parts,
        })
    }
}

impl<P: SteeringPlanDao, Q: SteeringPartDao> ReadModelUpdater for SteeringReadModelUpdater<P, Q> {
    type Error = ReadModelUpdateError;

    /// 参照入力が動いていれば steering の 2 表を作り直す。
    ///
    /// 読むのは毎回である — 規則ファイルの編集はイベントを伴わないので、「動いたかどうか」を
    /// 読まずに知る手立てが無い。読んだうえで `source_digest` を保存済みの値と比べ、
    /// **同じなら 1 行も書かない**。毎回書き替えると、規則を 1 文字も触っていないのに
    /// 束のバイトが動きうる。
    ///
    /// 最初の比較は書込ロックを取らずに行う (規則を触っていない更新はロックを取り合わない)。
    /// 動いていれば `BEGIN IMMEDIATE` で書込ロックを取ってから比べ直す — 比べてからロックを
    /// 取るまでの間に別の書き手が同じ参照入力を書いていれば、書き直さない。IMMEDIATE で
    /// 開くのは、読んでから書くトランザクションを DEFERRED で始めると、別の書き手がいるときの
    /// 書込昇格が busy timeout を待たずに即失敗するからである (#134)。
    ///
    /// # Errors
    ///
    /// 規則ファイルが在るのに読めない (`SteeringRead`)・刻めない (`SteeringPack`)、表の
    /// 読み書きやトランザクションの確定に失敗した (`Read`) 場合。
    async fn update_read_models(&mut self) -> Result<(), ReadModelUpdateError> {
        let rules = self.source.read()?;
        let source_digest = rules.source_digest();
        if self.plans.find_source_digest(&self.connection)?.as_deref()
            == Some(source_digest.as_str())
        {
            return Ok(());
        }
        let tables = SteeringTables::pack(&rules)?;
        let mut transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .at_store(&self.path)?;
        if self.plans.find_source_digest(&transaction)?.as_deref() == Some(tables.source_digest()) {
            return Ok(());
        }
        self.plans
            .replace(&mut transaction, tables.plans(), tables.source_digest())?;
        self.parts.replace(&mut transaction, tables.parts())?;
        transaction.commit().at_store(&self.path)?;
        Ok(())
    }
}
