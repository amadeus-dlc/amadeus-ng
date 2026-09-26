//! `CodeGenerationApprovalDao` の SQLite 実装 — `read_code_generation_approval` 表 1 つだけを
//! 読み書きする。
//!
//! 行を書いて読み戻す試験は `tests/reference_surface_updater_contract.rs` にある — 実際の
//! 投影 ([`crate::read_tables::CodeGenerationApprovalTables`]) が組んだ行を実ストアの履歴から
//! 用意できる結合試験の側で見る。

use rusqlite::{Connection, OptionalExtension as _, Transaction, params};

use super::journal_reader_impl::corrupt_error;
use super::store_failure::SqliteResultExt;
use super::{
    CodeGenerationApprovalDao, CodeGenerationApprovalRow, CorruptCause, GlobalSeqNr,
    JournalReadError, SourceStamp,
};

/// 表の DDL (冪等)。主キーは (`execution_id`, `target_id`) から導いた代理キーで、自然キーの
/// 重複は UNIQUE 制約が止める。
const CREATE_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS read_code_generation_approval (
    id TEXT PRIMARY KEY,
    execution_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    ok INTEGER NOT NULL,
    reason TEXT NOT NULL,
    unit TEXT,
    contract_hash TEXT,
    source_digest TEXT NOT NULL,
    as_of INTEGER NOT NULL,
    UNIQUE(execution_id,target_id)
);";

/// 行が名乗る出所。
const SELECT_STAMP: &str =
    "SELECT source_digest, as_of FROM read_code_generation_approval WHERE id = ?1";

/// 主キーの行を消す (差し替えの前半)。
const DELETE: &str = "DELETE FROM read_code_generation_approval WHERE id = ?1";

/// 行を足す。
const INSERT: &str = "INSERT INTO read_code_generation_approval
     (id, execution_id, target_id, ok, reason, unit, contract_hash, source_digest, as_of)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)";

/// 表を落とす (索引も一緒に落ちる)。
const DROP_TABLE: &str = "DROP TABLE IF EXISTS read_code_generation_approval";

/// 表が在るか (`sqlite_master` を引くだけ — 書込ロックを取らない)。
const TABLE_EXISTS: &str = "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'read_code_generation_approval'";

/// `read_code_generation_approval` 表の DAO の実装。状態を持たない。
#[derive(Debug, Clone, Copy, Default)]
pub struct CodeGenerationApprovalDaoImpl;

impl CodeGenerationApprovalDao for CodeGenerationApprovalDaoImpl {
    fn create_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError> {
        transaction
            .execute_batch(CREATE_TABLE)
            .at_connection(transaction)
    }

    fn drop_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError> {
        transaction
            .execute_batch(DROP_TABLE)
            .at_connection(transaction)
    }

    fn table_exists(&self, connection: &Connection) -> Result<bool, JournalReadError> {
        let found: i64 = connection
            .query_row(TABLE_EXISTS, [], |row| row.get(0))
            .at_connection(connection)?;
        Ok(found > 0)
    }

    fn find_stamp(
        &self,
        connection: &Connection,
        id: &str,
    ) -> Result<Option<SourceStamp>, JournalReadError> {
        let saved: Option<(String, i64)> = connection
            .query_row(SELECT_STAMP, [id], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional()
            .at_connection(connection)?;
        saved
            .map(|(source_digest, as_of)| {
                u64::try_from(as_of)
                    .map(|as_of| SourceStamp::new(source_digest, GlobalSeqNr::new(as_of)))
                    .map_err(|_| corrupt_error(id, None, CorruptCause::InvariantViolation))
            })
            .transpose()
    }

    fn save(
        &self,
        transaction: &mut Transaction<'_>,
        row: &CodeGenerationApprovalRow,
    ) -> Result<(), JournalReadError> {
        let as_of = i64::try_from(row.as_of().to_u64())
            .map_err(|_| corrupt_error(row.id(), None, CorruptCause::InvariantViolation))?;
        transaction
            .execute(DELETE, [row.id()])
            .at_connection(transaction)?;
        transaction
            .execute(
                INSERT,
                params![
                    row.id(),
                    row.execution_id(),
                    row.target_id(),
                    i64::from(row.ok()),
                    row.reason(),
                    row.unit(),
                    row.contract_hash(),
                    row.source_digest(),
                    as_of,
                ],
            )
            .at_connection(transaction)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn created(connection: &mut Connection) {
        let mut transaction = connection.transaction().unwrap();
        CodeGenerationApprovalDaoImpl
            .create_table(&mut transaction)
            .unwrap();
        transaction.commit().unwrap();
    }

    #[test]
    fn a_missing_row_has_no_stamp_and_creating_twice_is_harmless() {
        let mut connection = Connection::open_in_memory().unwrap();
        created(&mut connection);
        created(&mut connection);
        assert_eq!(
            CodeGenerationApprovalDaoImpl
                .find_stamp(&connection, "id")
                .unwrap(),
            None
        );
    }

    #[test]
    fn a_saved_stamp_reads_back_and_a_negative_position_is_corrupt() {
        let mut connection = Connection::open_in_memory().unwrap();
        created(&mut connection);
        connection
            .execute(
                "INSERT INTO read_code_generation_approval
                 VALUES ('a','e','t',0,'refused',NULL,NULL,'digest',7)",
                [],
            )
            .unwrap();
        assert_eq!(
            CodeGenerationApprovalDaoImpl
                .find_stamp(&connection, "a")
                .unwrap(),
            Some(SourceStamp::new("digest".to_string(), GlobalSeqNr::new(7)))
        );
        connection
            .execute("UPDATE read_code_generation_approval SET as_of = -1", [])
            .unwrap();
        assert_eq!(
            CodeGenerationApprovalDaoImpl
                .find_stamp(&connection, "a")
                .unwrap_err(),
            corrupt_error("a", None, CorruptCause::InvariantViolation)
        );
    }
}
