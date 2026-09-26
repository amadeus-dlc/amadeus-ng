//! `DefinitionScopePhaseEntryDao` の SQLite 実装 — `read_definition_scope_phase_entry` 表 1 つだけを読み書きする。

use rusqlite::{Connection, Transaction, params};

use super::column_value::{count_schema_objects, read_content, scan_position};
use super::store_failure::SqliteResultExt;
use super::{
    DefinitionScopePhaseEntryDao, DefinitionScopePhaseEntryRow, GlobalSeqNr, JournalReadError,
    TableContent,
};

/// 表と索引の DDL (冪等)。
///
/// 列の並びは内容の照合 ([`DefinitionScopePhaseEntryDao::find_content`]) の並びでもある。列を足す・落とす・型を
/// 変えるときは、読み面の版 (`read_model_schema::READ_SCHEMA_VERSION`) を上げる。
///
/// 索引は表と同じ口で作る — 表だけ在って索引が無い断面を作ると、自然キーの重複が静かに通るか、
/// クエリ側の引当が全走査になる。
const CREATE_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS read_definition_scope_phase_entry (
  id               TEXT    PRIMARY KEY,
  definition_id    TEXT    NOT NULL,
  scope            TEXT    NOT NULL,
  phase            TEXT    NOT NULL,
  first_stage_slug TEXT    NOT NULL,
  as_of            INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS read_definition_scope_phase_entry_key
  ON read_definition_scope_phase_entry(definition_id, scope, phase);";

/// 表と索引が揃っているか (`sqlite_master` を引くだけ — 書込ロックを取らない)。数えた本数が
/// [`SCHEMA_OBJECTS`] に届いていれば揃っている。
const TABLE_EXISTS: &str = "SELECT COUNT(*) FROM sqlite_master WHERE (type = 'table' AND name = 'read_definition_scope_phase_entry') OR (type = 'index' AND name IN ('read_definition_scope_phase_entry_key'))";

/// 揃っているべき本数 (表 1 つと、その索引)。
const SCHEMA_OBJECTS: i64 = 2;

/// 表を落とす (索引も一緒に落ちる)。
const DROP_TABLE: &str = "DROP TABLE IF EXISTS read_definition_scope_phase_entry";

/// 全行を消す (全差し替えの前半)。
const DELETE_ALL: &str = "DELETE FROM read_definition_scope_phase_entry";

/// 行を足す。
const WRITE: &str = "INSERT INTO read_definition_scope_phase_entry
             (id, definition_id, scope, phase, first_stage_slug, as_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)";

/// 全行の生の値 (主キーの昇順)。
const SELECT_CONTENT: &str = "SELECT * FROM read_definition_scope_phase_entry ORDER BY id";

/// `read_definition_scope_phase_entry` 表の DAO の実装。状態を持たない。
#[derive(Debug, Clone, Copy, Default)]
pub struct DefinitionScopePhaseEntryDaoImpl;

impl DefinitionScopePhaseEntryDao for DefinitionScopePhaseEntryDaoImpl {
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
        rows: &[DefinitionScopePhaseEntryRow],
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
                        row.scope(),
                        row.phase(),
                        row.first_stage_slug(),
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
