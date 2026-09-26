//! `amadeus_read_model_head` 表の DAO — 共有構造化面の公開位置と内容の記録の保存先。

use rusqlite::{Connection, Transaction};

use super::ReadModelHeadRow;
use crate::orchestration::JournalReadError;

/// `amadeus_read_model_head` 表 (1 行だけの表) の DAO。
///
/// 単一テーブルの I/O だけを持つ。記録と 20 表の内容を照らし合わせる (ダイジェストを
/// 計算して比べる・未照合の記録を歴史から描き直した行と比べる) のは表をまたぐ検査なので、
/// 更新器が持つ。名前から名前空間の接頭辞 `amadeus_` を除くのは `read_` と同じ理由である。
///
/// 書込は更新器が開いたトランザクションを `&mut` で受け取る。DAO 自身は状態を持たない。
pub trait ReadModelHeadDao {
    /// 表が無ければ作る (冪等)。行は作らない — 初期値を置くのは更新器である。
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

    /// 記録の行。まだ置かれていなければ `None`。
    ///
    /// # Errors
    ///
    /// 読めない・列の型が違う場合 (`Io`)。
    fn find(&self, connection: &Connection) -> Result<Option<ReadModelHeadRow>, JournalReadError>;

    /// 記録の行を保存する (無ければ足し、在れば全列を上書きする)。
    ///
    /// # Errors
    ///
    /// 書けない・表の制約 (位置は 0 以上、世代は正) に反する場合 (`Io`)。
    fn save(
        &self,
        transaction: &mut Transaction<'_>,
        row: &ReadModelHeadRow,
    ) -> Result<(), JournalReadError>;
}
