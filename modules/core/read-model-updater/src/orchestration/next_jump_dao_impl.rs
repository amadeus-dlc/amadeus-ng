//! `NextJumpDao` の SQLite 実装 — `read_next_jump` 表 1 つだけを読み書きする。

use rusqlite::{Connection, Transaction, params};

use super::column_value::{count_schema_objects, integer, read_content, scan_position};
use super::store_failure::SqliteResultExt;
use super::{GlobalSeqNr, JournalReadError, NextJumpDao, NextJumpRow, TableContent};

/// 表と索引の DDL (冪等)。
///
/// 列の並びは内容の照合 ([`NextJumpDao::find_content`]) の並びでもある。列を足す・落とす・型を
/// 変えるときは、読み面の版 (`read_model_schema::READ_SCHEMA_VERSION`) を上げる。
///
/// 索引は表と同じ口で作る — 表だけ在って索引が無い断面を作ると、自然キーの重複が静かに通るか、
/// クエリ側の引当が全走査になる。
const CREATE_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS read_next_jump (
  id           TEXT    PRIMARY KEY,
  execution_id TEXT    NOT NULL,
  target_index INTEGER NOT NULL,
  target_slug  TEXT    NOT NULL,
  outcome      TEXT    NOT NULL,
  refusal      TEXT,
  resolution   TEXT,
  as_of        INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS read_next_jump_key
  ON read_next_jump(execution_id, target_index);
CREATE INDEX IF NOT EXISTS read_next_jump_target_slug
  ON read_next_jump(execution_id, target_slug);";

/// 表と索引が揃っているか (`sqlite_master` を引くだけ — 書込ロックを取らない)。数えた本数が
/// [`SCHEMA_OBJECTS`] に届いていれば揃っている。
const TABLE_EXISTS: &str = "SELECT COUNT(*) FROM sqlite_master WHERE (type = 'table' AND name = 'read_next_jump') OR (type = 'index' AND name IN ('read_next_jump_key', 'read_next_jump_target_slug'))";

/// 揃っているべき本数 (表 1 つと、その索引)。
const SCHEMA_OBJECTS: i64 = 3;

/// 表を落とす (索引も一緒に落ちる)。
const DROP_TABLE: &str = "DROP TABLE IF EXISTS read_next_jump";

/// 全行を消す (全差し替えの前半)。
const DELETE_ALL: &str = "DELETE FROM read_next_jump";

/// 行を足す。
const WRITE: &str = "INSERT INTO read_next_jump
             (id, execution_id, target_index, target_slug, outcome, refusal, resolution, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)";

/// 全行の生の値 (主キーの昇順)。
const SELECT_CONTENT: &str = "SELECT * FROM read_next_jump ORDER BY id";

/// `read_next_jump` 表の DAO の実装。状態を持たない。
#[derive(Debug, Clone, Copy, Default)]
pub struct NextJumpDaoImpl;

impl NextJumpDao for NextJumpDaoImpl {
    fn create_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError> {
        transaction
            .execute_batch(CREATE_TABLE)
            .at_connection(transaction)
    }

    fn table_exists(&self, connection: &Connection) -> Result<bool, JournalReadError> {
        Ok(count_schema_objects(connection, TABLE_EXISTS)? == SCHEMA_OBJECTS)
    }

    fn drop_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError> {
        transaction
            .execute_batch(DROP_TABLE)
            .at_connection(transaction)
    }

    fn delete_all(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError> {
        transaction
            .execute(DELETE_ALL, [])
            .at_connection(transaction)?;
        Ok(())
    }

    fn insert(
        &self,
        transaction: &mut Transaction<'_>,
        rows: &[NextJumpRow],
        as_of: GlobalSeqNr,
    ) -> Result<(), JournalReadError> {
        let as_of = scan_position(as_of)?;
        for row in rows {
            transaction
                .execute(
                    WRITE,
                    params![
                        row.id(),
                        row.execution_id(),
                        integer(row.target_index(), row.id())?,
                        row.target_slug(),
                        row.outcome(),
                        row.refusal(),
                        row.resolution(),
                        as_of
                    ],
                )
                .at_connection(transaction)?;
        }
        Ok(())
    }

    fn find_content(&self, connection: &Connection) -> Result<TableContent, JournalReadError> {
        read_content(connection, SELECT_CONTENT)
    }
}
