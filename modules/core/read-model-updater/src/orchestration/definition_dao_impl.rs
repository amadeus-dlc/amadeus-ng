//! `DefinitionDao` の SQLite 実装 — `read_definition` 表 1 つだけを読み書きする。

use rusqlite::{Connection, Transaction, params};

use super::column_value::{
    count_schema_objects, integer, max_position, read_content, scan_position,
};
use super::store_failure::SqliteResultExt;
use super::{DefinitionDao, DefinitionRow, GlobalSeqNr, JournalReadError, TableContent};

/// 表の DDL (冪等)。
///
/// 列の並びは内容の照合 ([`DefinitionDao::find_content`]) の並びでもある。列を足す・落とす・型を
/// 変えるときは、読み面の版 (`read_model_schema::READ_SCHEMA_VERSION`) を上げる。
const CREATE_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS read_definition (
  id          TEXT PRIMARY KEY,
  revision    TEXT    NOT NULL,
  stage_count INTEGER NOT NULL,
  scope_count INTEGER NOT NULL,
  as_of       INTEGER NOT NULL
);";

/// 表と索引が揃っているか (`sqlite_master` を引くだけ — 書込ロックを取らない)。数えた本数が
/// [`SCHEMA_OBJECTS`] に届いていれば揃っている。
const TABLE_EXISTS: &str =
    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'read_definition'";

/// 揃っているべき本数 (表 1 つと、その索引)。
const SCHEMA_OBJECTS: i64 = 1;

/// 表を落とす (索引も一緒に落ちる)。
const DROP_TABLE: &str = "DROP TABLE IF EXISTS read_definition";

/// 全行を消す (全差し替えの前半)。
const DELETE_ALL: &str = "DELETE FROM read_definition";

/// 行を足す。
const WRITE: &str = "INSERT INTO read_definition
     (id, revision, stage_count, scope_count, as_of)
     VALUES (?1, ?2, ?3, ?4, ?5)";

/// 全行の生の値 (主キーの昇順)。
const SELECT_CONTENT: &str = "SELECT * FROM read_definition ORDER BY id";

/// `as_of` 列の最大値。
const SELECT_MAX_AS_OF: &str = "SELECT MAX(as_of) FROM read_definition";

/// `read_definition` 表の DAO の実装。状態を持たない。
#[derive(Debug, Clone, Copy, Default)]
pub struct DefinitionDaoImpl;

impl DefinitionDao for DefinitionDaoImpl {
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
        rows: &[DefinitionRow],
        as_of: GlobalSeqNr,
    ) -> Result<(), JournalReadError> {
        let as_of = scan_position(as_of)?;
        for row in rows {
            transaction
                .execute(
                    WRITE,
                    params![
                        row.id(),
                        row.revision(),
                        integer(row.stage_count(), row.id())?,
                        integer(row.scope_count(), row.id())?,
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

    fn find_max_as_of(
        &self,
        connection: &Connection,
    ) -> Result<Option<GlobalSeqNr>, JournalReadError> {
        max_position(connection, SELECT_MAX_AS_OF)
    }
}
