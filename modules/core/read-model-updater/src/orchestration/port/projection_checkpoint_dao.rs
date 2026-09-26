//! `amadeus_projection_checkpoint` 表の DAO — 処理したシーケンス番号の保存先。

use rusqlite::{Connection, Transaction};

use super::ProjectionCheckpointRow;
use crate::orchestration::{GlobalSeqNr, JournalReadError, ProjectionName};

/// `amadeus_projection_checkpoint` 表の DAO。
///
/// **処理したシーケンス番号はリードモデル側の状態である** (オーナー裁定 2026-09-26)。
/// 構造化面の更新器は保存した番号より後の事実を読み、表を更新し、処理した番号をこの DAO で
/// 保存する。表と番号は同じ IMMEDIATE トランザクションで確定する (原則 6)。
///
/// 単一テーブルの I/O だけを持つ。前進の単調性 (後ろへ戻さない) と、保存したアンカーを
/// ジャーナルの行と照らし合わせることは、ジャーナルとこの表をまたぐ検査なので更新器が持つ。
/// 名前から名前空間の接頭辞 `amadeus_` を除くのは、`read_` を除くのと同じ理由である
/// (本家の表と衝突しないための接頭辞であって、表の意味ではない)。
///
/// 書込は更新器が開いたトランザクションを `&mut` で受け取る。DAO 自身は状態を持たない。
pub trait ProjectionCheckpointDao {
    /// 表が無ければ作る (冪等)。
    ///
    /// # Errors
    ///
    /// 表を作れない場合 (`Io`)。
    fn create_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError>;

    /// 表が在るか (`sqlite_master` の読取だけで、書込ロックを取らない)。
    ///
    /// # Errors
    ///
    /// 読めない場合 (`Io`)。
    fn table_exists(&self, connection: &Connection) -> Result<bool, JournalReadError>;

    /// 投影 `projection` の行。未登録なら `None`。
    ///
    /// アンカーは集約 ID と通番の両方が保存されているときだけ返す (片方でも欠けていれば
    /// `None` — それをどう扱うかは更新器が決める)。
    ///
    /// # Errors
    ///
    /// 読めない・列の型が違う (`Io`)、保存値が負 (`Corrupt`) の場合。
    fn find(
        &self,
        connection: &Connection,
        projection: &ProjectionName,
    ) -> Result<Option<ProjectionCheckpointRow>, JournalReadError>;

    /// 全投影のうち最も進んだ位置 (1 行も無ければ `None`)。
    ///
    /// # Errors
    ///
    /// 読めない (`Io`)、保存値が負 (`Corrupt`) の場合。
    fn find_max_position(
        &self,
        connection: &Connection,
    ) -> Result<Option<GlobalSeqNr>, JournalReadError>;

    /// 行を保存する (未登録なら足し、登録済みなら位置とアンカーを上書きする)。
    ///
    /// # Errors
    ///
    /// 書けない (`Io`)、値が列に収まらない (`Corrupt`) 場合。
    fn save(
        &self,
        transaction: &mut Transaction<'_>,
        row: &ProjectionCheckpointRow,
    ) -> Result<(), JournalReadError>;
}
