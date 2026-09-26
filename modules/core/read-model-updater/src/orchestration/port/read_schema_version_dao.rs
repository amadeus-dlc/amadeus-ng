//! 読み面スキーマの版 (`PRAGMA user_version`) の DAO。

use rusqlite::{Connection, Transaction};

use crate::orchestration::JournalReadError;

/// 読み面スキーマの版の DAO — 保存先は SQLite のヘッダが持つ 32bit の欄 `PRAGMA user_version`。
///
/// 表ではないが、値 1 つだけの表とみなして DAO を 1 本立てる (ファイルを 1 表とみなすのと
/// 同じ扱い — `coding-rules/read-model-updater-structure.md` 原則 3)。本家のイベントストアも
/// 我々のほかの表もこの欄を使っていない (実測)。
///
/// 版が動いたときに `read_*` 表をどう作り直すかは、読み面の表の用意を持つ 1 か所
/// (`read_model_schema::prepare`) が決める。この DAO は値を読み書きするだけである。
pub trait ReadSchemaVersionDao {
    /// 保存されている版 (未設定の DB は `0`)。
    ///
    /// # Errors
    ///
    /// 読めない場合 (`Io`)。
    fn find(&self, connection: &Connection) -> Result<i64, JournalReadError>;

    /// 版を保存する。
    ///
    /// # Errors
    ///
    /// 書けない場合 (`Io`)。
    fn save(&self, transaction: &mut Transaction<'_>, version: i64)
    -> Result<(), JournalReadError>;
}
