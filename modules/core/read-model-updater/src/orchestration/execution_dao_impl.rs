//! `ExecutionDao` の SQLite 実装 — `read_execution` 表 1 つだけを読み書きする。

use rusqlite::{Connection, Transaction, params};

use super::column_value::{
    count_schema_objects, integer, max_position, optional_integer, read_content, scan_position,
};
use super::store_failure::SqliteResultExt;
use super::{ExecutionDao, ExecutionRow, GlobalSeqNr, JournalReadError, TableContent};

/// 表と索引の DDL (冪等)。
///
/// 列の並びは内容の照合 ([`ExecutionDao::find_content`]) の並びでもある。列を足す・落とす・型を
/// 変えるときは、読み面の版 (`read_model_schema::READ_SCHEMA_VERSION`) を上げる。
///
/// 索引は表と同じ口で作る — 表だけ在って索引が無い断面を作ると、自然キーの重複が静かに通るか、
/// クエリ側の引当が全走査になる。
const CREATE_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS read_execution (
  id               TEXT PRIMARY KEY,
  intent_id        TEXT    NOT NULL,
  scope            TEXT    NOT NULL,
  status           TEXT    NOT NULL,
  cursor_index     INTEGER,
  cursor_slug      TEXT,
  parked_at_index  INTEGER,
  parked_at_slug   TEXT,
  parked_active    INTEGER NOT NULL,
  accepts_commands INTEGER NOT NULL,
  autonomy         TEXT    NOT NULL,
  skeleton_stance  TEXT,
  seq_nr           INTEGER NOT NULL,
  last_updated_at  TEXT    NOT NULL,
  state_binding    TEXT    NOT NULL,
  first_substantive_run INTEGER NOT NULL,
  continuation_wait TEXT,
  as_of            INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS read_execution_intent_id
  ON read_execution(intent_id);
CREATE INDEX IF NOT EXISTS read_execution_state_binding
  ON read_execution(state_binding);";

/// 表と索引が揃っているか (`sqlite_master` を引くだけ — 書込ロックを取らない)。数えた本数が
/// [`SCHEMA_OBJECTS`] に届いていれば揃っている。
const TABLE_EXISTS: &str = "SELECT COUNT(*) FROM sqlite_master WHERE (type = 'table' AND name = 'read_execution') OR (type = 'index' AND name IN ('read_execution_intent_id', 'read_execution_state_binding'))";

/// 揃っているべき本数 (表 1 つと、その索引)。
const SCHEMA_OBJECTS: i64 = 3;

/// 表を落とす (索引も一緒に落ちる)。
const DROP_TABLE: &str = "DROP TABLE IF EXISTS read_execution";

/// 全行を消す (全差し替えの前半)。
const DELETE_ALL: &str = "DELETE FROM read_execution";

/// 行を足す。
const WRITE: &str = "INSERT INTO read_execution
             (id, intent_id, scope, status, cursor_index, cursor_slug,
              parked_at_index, parked_at_slug, parked_active, accepts_commands, autonomy,
              skeleton_stance, seq_nr, last_updated_at, state_binding, as_of, first_substantive_run, continuation_wait)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)";

/// 全行の生の値 (主キーの昇順)。
const SELECT_CONTENT: &str = "SELECT * FROM read_execution ORDER BY id";

/// `as_of` 列の最大値。
const SELECT_MAX_AS_OF: &str = "SELECT MAX(as_of) FROM read_execution";

/// `read_execution` 表の DAO の実装。状態を持たない。
#[derive(Debug, Clone, Copy, Default)]
pub struct ExecutionDaoImpl;

impl ExecutionDao for ExecutionDaoImpl {
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
        rows: &[ExecutionRow],
        as_of: GlobalSeqNr,
    ) -> Result<(), JournalReadError> {
        let as_of = scan_position(as_of)?;
        for row in rows {
            transaction
                .execute(
                    WRITE,
                    params![
                        row.id(),
                        row.intent_id(),
                        row.scope(),
                        row.status(),
                        optional_integer(row.cursor_index(), row.id())?,
                        row.cursor_slug(),
                        optional_integer(row.parked_at_index(), row.id())?,
                        row.parked_at_slug(),
                        row.parked_active(),
                        row.accepts_commands(),
                        row.autonomy(),
                        row.skeleton_stance(),
                        integer(row.seq_nr(), row.id())?,
                        row.last_updated_at(),
                        row.state_binding(),
                        as_of,
                        row.first_substantive_run(),
                        row.continuation_wait()
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
