//! `read_doctor_check` 表の DAO — この表 1 つの I/O だけを持つ。

use rusqlite::Transaction;

use super::DoctorCheckRow;
use crate::orchestration::JournalReadError;

/// `read_doctor_check` 表の DAO。
///
/// 単一テーブルの I/O だけを持つ。行が指す `read_doctor_report` の存在は検査しない —
/// 両表の対を揃えるのは、同じトランザクションで両方を書く更新器の仕事である。
///
/// 書込は更新器が開いたトランザクションを `&mut` で受け取る。DAO 自身は状態を持たない。
pub trait DoctorCheckDao {
    /// 表と索引が無ければ作る (冪等)。
    ///
    /// # Errors
    ///
    /// 表を作れない場合 (`Io`)。
    fn create_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError>;

    /// FK 列 `report_id` が `report_id` の行をすべて `rows` に差し替える。
    ///
    /// # Errors
    ///
    /// 書けない (`Io`)、値が列に収まらない (`Corrupt`) 場合。
    fn replace_for_report(
        &self,
        transaction: &mut Transaction<'_>,
        report_id: &str,
        rows: &[DoctorCheckRow],
    ) -> Result<(), JournalReadError>;
}
