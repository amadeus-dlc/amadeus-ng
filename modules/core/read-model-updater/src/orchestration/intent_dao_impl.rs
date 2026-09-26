//! `IntentDao` の SQLite 実装 — `read_intent` 表 1 つだけを読み書きする。

use rusqlite::{Connection, Transaction, params};

use super::column_value::{
    count_schema_objects, integer, max_position, read_content, scan_position,
};
use super::store_failure::SqliteResultExt;
use super::{GlobalSeqNr, IntentDao, IntentRow, JournalReadError, TableContent};

/// 表と索引の DDL (冪等)。
///
/// 列の並びは内容の照合 ([`IntentDao::find_content`]) の並びでもある。列を足す・落とす・型を
/// 変えるときは、読み面の版 (`read_model_schema::READ_SCHEMA_VERSION`) を上げる。
///
/// 索引は表と同じ口で作る — 表だけ在って索引が無い断面を作ると、自然キーの重複が静かに通るか、
/// クエリ側の引当が全走査になる。
const CREATE_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS read_intent (
  execute_count INTEGER NOT NULL,
  first_stage TEXT,
  first_phase TEXT,
  id                  TEXT PRIMARY KEY,
  definition_id       TEXT    NOT NULL,
  definition_revision TEXT    NOT NULL,
  scope               TEXT    NOT NULL,
  request             TEXT    NOT NULL,
  depth               TEXT,
  test_strategy       TEXT,
  review              TEXT,
  created_at          TEXT    NOT NULL,
  project_type        TEXT    NOT NULL,
  project_kind        TEXT    NOT NULL,
  languages           TEXT    NOT NULL,
  frameworks          TEXT    NOT NULL,
  build_system        TEXT    NOT NULL,
  as_of               INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS read_intent_definition_id
  ON read_intent(definition_id);";

/// 表と索引が揃っているか (`sqlite_master` を引くだけ — 書込ロックを取らない)。数えた本数が
/// [`SCHEMA_OBJECTS`] に届いていれば揃っている。
const TABLE_EXISTS: &str = "SELECT COUNT(*) FROM sqlite_master WHERE (type = 'table' AND name = 'read_intent') OR (type = 'index' AND name IN ('read_intent_definition_id'))";

/// 揃っているべき本数 (表 1 つと、その索引)。
const SCHEMA_OBJECTS: i64 = 2;

/// 表を落とす (索引も一緒に落ちる)。
const DROP_TABLE: &str = "DROP TABLE IF EXISTS read_intent";

/// 全行を消す (全差し替えの前半)。
const DELETE_ALL: &str = "DELETE FROM read_intent";

/// 行を足す。
const WRITE: &str = "INSERT INTO read_intent
             (id, definition_id, definition_revision, scope, request, depth,
              test_strategy, review, created_at, project_type, project_kind, languages,
              frameworks, build_system, as_of, execute_count, first_stage, first_phase)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)";

/// 全行の生の値 (主キーの昇順)。
const SELECT_CONTENT: &str = "SELECT * FROM read_intent ORDER BY id";

/// `as_of` 列の最大値。
const SELECT_MAX_AS_OF: &str = "SELECT MAX(as_of) FROM read_intent";

/// `read_intent` 表の DAO の実装。状態を持たない。
#[derive(Debug, Clone, Copy, Default)]
pub struct IntentDaoImpl;

impl IntentDao for IntentDaoImpl {
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
        rows: &[IntentRow],
        as_of: GlobalSeqNr,
    ) -> Result<(), JournalReadError> {
        let as_of = scan_position(as_of)?;
        for row in rows {
            transaction
                .execute(
                    WRITE,
                    params![
                        row.id(),
                        row.definition_id(),
                        row.definition_revision(),
                        row.scope(),
                        row.request(),
                        row.depth(),
                        row.test_strategy(),
                        row.review(),
                        row.created_at(),
                        row.project_type(),
                        row.project_kind(),
                        row.languages(),
                        row.frameworks(),
                        row.build_system(),
                        as_of,
                        integer(row.execute_count(), row.id())?,
                        row.first_stage(),
                        row.first_phase()
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
