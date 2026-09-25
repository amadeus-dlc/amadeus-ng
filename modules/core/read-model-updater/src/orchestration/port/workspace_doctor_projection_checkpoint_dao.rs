//! `workspace_doctor_projection_checkpoint` 表の DAO — 処理したシーケンス番号の保存先。

use rusqlite::{Connection, Transaction};

use crate::orchestration::{GlobalSeqNr, JournalReadError, ProjectionName};

/// `workspace_doctor_projection_checkpoint` 表の DAO。
///
/// **処理したシーケンス番号はリードモデル側の状態である** (オーナー裁定 2026-09-26)。
/// 更新器は保存した番号より後の事実を読み、表を更新し、処理した番号をこの DAO で保存する。
/// 再起動時はその番号の次から始める。
///
/// 番号の単調性 (後ろへ戻さない) は更新器が構成で守る — 保存するのは、保存済みの番号より
/// 後から読んだ事実の位置だけである。
pub trait WorkspaceDoctorProjectionCheckpointDao {
    /// 表が無ければ作る (冪等)。
    ///
    /// # Errors
    ///
    /// 表を作れない場合 (`Io`)。
    fn create_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError>;

    /// 投影が処理したシーケンス番号を引く。未登録なら [`GlobalSeqNr::ZERO`]。
    ///
    /// # Errors
    ///
    /// 読めない (`Io`)、保存値が負 (`Corrupt`) の場合。
    fn find(
        &self,
        connection: &Connection,
        projection: &ProjectionName,
    ) -> Result<GlobalSeqNr, JournalReadError>;

    /// 投影が処理したシーケンス番号を保存する (未登録なら足す)。
    ///
    /// # Errors
    ///
    /// 書けない (`Io`)、値が列に収まらない (`Corrupt`) 場合。
    fn save(
        &self,
        transaction: &mut Transaction<'_>,
        projection: &ProjectionName,
        position: GlobalSeqNr,
    ) -> Result<(), JournalReadError>;
}
