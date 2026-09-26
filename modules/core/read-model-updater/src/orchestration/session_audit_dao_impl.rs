//! `SessionAuditDao` の SQLite 実装 — `read_session_audit` 表 1 つだけを読み書きする。

use rusqlite::{Connection, Transaction, params};

use super::column_value::{count_schema_objects, read_content, scan_position};
use super::store_failure::SqliteResultExt;
use super::{GlobalSeqNr, JournalReadError, SessionAuditDao, SessionAuditRow, TableContent};

/// 表の DDL (冪等)。
///
/// 列の並びは内容の照合 ([`SessionAuditDao::find_content`]) の並びでもある。列を足す・落とす・型を
/// 変えるときは、読み面の版 (`read_model_schema::READ_SCHEMA_VERSION`) を上げる。
const CREATE_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS read_session_audit (id TEXT PRIMARY KEY,aggregate_id TEXT NOT NULL,target TEXT NOT NULL,kind TEXT NOT NULL,as_of INTEGER NOT NULL);";

/// 表と索引が揃っているか (`sqlite_master` を引くだけ — 書込ロックを取らない)。数えた本数が
/// [`SCHEMA_OBJECTS`] に届いていれば揃っている。
const TABLE_EXISTS: &str =
    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'read_session_audit'";

/// 揃っているべき本数 (表 1 つと、その索引)。
const SCHEMA_OBJECTS: i64 = 1;

/// 表を落とす (索引も一緒に落ちる)。
const DROP_TABLE: &str = "DROP TABLE IF EXISTS read_session_audit";

/// 全行を消す (全差し替えの前半)。
const DELETE_ALL: &str = "DELETE FROM read_session_audit";

/// 行を保存する (主キーが同じ行は上書き)。
const WRITE: &str = "INSERT INTO read_session_audit (id,aggregate_id,target,kind,as_of) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET aggregate_id=excluded.aggregate_id,target=excluded.target,kind=excluded.kind,as_of=excluded.as_of";

/// 全行の生の値 (主キーの昇順)。
const SELECT_CONTENT: &str = "SELECT * FROM read_session_audit ORDER BY id";

/// `read_session_audit` 表の DAO の実装。状態を持たない。
#[derive(Debug, Clone, Copy, Default)]
pub struct SessionAuditDaoImpl;

impl SessionAuditDao for SessionAuditDaoImpl {
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

    fn save(
        &self,
        transaction: &mut Transaction<'_>,
        rows: &[SessionAuditRow],
        as_of: GlobalSeqNr,
    ) -> Result<(), JournalReadError> {
        let as_of = scan_position(as_of)?;
        for row in rows {
            transaction
                .execute(
                    WRITE,
                    params![
                        row.id(),
                        row.aggregate_id(),
                        row.target(),
                        row.kind(),
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
