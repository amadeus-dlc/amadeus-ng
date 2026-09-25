//! `read_doctor_report` 表の DAO — この表 1 つの I/O だけを持つ。

use rusqlite::Transaction;

use super::DoctorReportRow;
use crate::orchestration::JournalReadError;

/// `read_doctor_report` 表の DAO。
///
/// 単一テーブルの I/O だけを持つ (オーナー裁定 2026-09-26「DAO はテーブル単一の I/O しか
/// できないのです！」)。`read_doctor_check` の行や処理したシーケンス番号には触れない — 両者を
/// 同じトランザクションで確定するのは更新器の仕事である。
///
/// 書込は更新器が開いたトランザクションを `&mut` で受け取る。DAO 自身は状態を持たない。
pub trait DoctorReportDao {
    /// 表と索引が無ければ作る (冪等)。
    ///
    /// # Errors
    ///
    /// 表を作れない場合 (`Io`)。
    fn create_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError>;

    /// 行を主キーで差し替える (無ければ足す)。
    ///
    /// # Errors
    ///
    /// 書けない (`Io`)、値が列に収まらない (`Corrupt`) 場合。
    fn save(
        &self,
        transaction: &mut Transaction<'_>,
        row: &DoctorReportRow,
    ) -> Result<(), JournalReadError>;
}
