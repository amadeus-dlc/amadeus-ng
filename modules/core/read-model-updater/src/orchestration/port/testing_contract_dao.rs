//! `read_testing_contract` 表の DAO — この表 1 つの I/O だけを持つ。

use rusqlite::{Connection, Transaction};

use super::{SourceStamp, TestingContractRow};
use crate::orchestration::{GlobalSeqNr, JournalReadError};

/// `read_testing_contract` 表の DAO。
///
/// 単一テーブルの I/O だけを持つ (オーナー裁定 2026-09-26)。保存済みの出所
/// ([`SourceStamp`]) といま組んだ行を比べて書くかどうかを決めるのは更新器
/// ([`crate::orchestration::TestingReadModelUpdater`]) であり、この DAO は値を読み書きする
/// だけである。
///
/// 書込は更新器が開いたトランザクションを `&mut` で受け取る。DAO 自身は状態を持たない。
pub trait TestingContractDao {
    /// 表が無ければ作る (冪等)。
    ///
    /// # Errors
    ///
    /// 表を作れない場合 (`Io`)。
    fn create_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError>;

    /// 主キー `id` の行が名乗る出所。行が無ければ `None`。
    ///
    /// # Errors
    ///
    /// 読めない (`Io`)、保存値が負 (`Corrupt`) の場合。
    fn find_stamp(
        &self,
        connection: &Connection,
        id: &str,
    ) -> Result<Option<SourceStamp>, JournalReadError>;

    /// 表の行をすべて `rows` に差し替える。`source_digest` と `as_of` は全行へ同じ値を
    /// 書く (どちらも断面全体の性質であり、行型には持たせない)。
    ///
    /// # Errors
    ///
    /// 書けない (`Io`)、値が列に収まらない (`Corrupt`) 場合。
    fn replace(
        &self,
        transaction: &mut Transaction<'_>,
        rows: &[TestingContractRow],
        source_digest: &str,
        as_of: GlobalSeqNr,
    ) -> Result<(), JournalReadError>;
}
