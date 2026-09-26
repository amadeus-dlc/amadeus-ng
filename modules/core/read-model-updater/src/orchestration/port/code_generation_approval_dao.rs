//! `read_code_generation_approval` 表の DAO — この表 1 つの I/O だけを持つ。

use rusqlite::{Connection, Transaction};

use super::{CodeGenerationApprovalRow, SourceStamp};
use crate::orchestration::JournalReadError;

/// `read_code_generation_approval` 表の DAO。
///
/// 単一テーブルの I/O だけを持つ (オーナー裁定 2026-09-26)。保存済みの出所
/// ([`SourceStamp`]) といま組んだ行を比べて書くかどうかを決めるのは更新器
/// ([`crate::orchestration::CodeGenerationApprovalReadModelUpdater`]) であり、この DAO は
/// 値を読み書きするだけである。
///
/// 書込は更新器が開いたトランザクションを `&mut` で受け取る。DAO 自身は状態を持たない。
pub trait CodeGenerationApprovalDao {
    /// 表が無ければ作る (冪等)。
    ///
    /// # Errors
    ///
    /// 表を作れない場合 (`Io`)。
    fn create_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError>;

    /// 表を落とす (索引も一緒に落ちる)。読み面の版が動いたときの作り直し
    /// (`read_model_schema::prepare`) だけが呼ぶ。
    ///
    /// # Errors
    ///
    /// 落とせない場合 (`Io`)。
    fn drop_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError>;

    /// 表が在るか (`sqlite_master` の読取だけで、書込ロックを取らない)。
    ///
    /// 更新器は開く段でこれを見て、表が揃っていれば書込トランザクションを開かない。
    /// 欠けているときだけ `BEGIN IMMEDIATE` の中で [`Self::create_table`] を呼ぶ。
    ///
    /// # Errors
    ///
    /// 読めない場合 (`Io`)。
    fn table_exists(&self, connection: &Connection) -> Result<bool, JournalReadError>;

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

    /// 主キー `id` の行を `row` に差し替える (無ければ足す)。
    ///
    /// # Errors
    ///
    /// 書けない (`Io`)、値が列に収まらない (`Corrupt`) 場合。
    fn save(
        &self,
        transaction: &mut Transaction<'_>,
        row: &CodeGenerationApprovalRow,
    ) -> Result<(), JournalReadError>;
}
