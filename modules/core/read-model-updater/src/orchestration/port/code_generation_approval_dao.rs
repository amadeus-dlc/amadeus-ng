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
