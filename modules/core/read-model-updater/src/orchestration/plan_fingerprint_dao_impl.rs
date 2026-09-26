//! `PlanFingerprintDao` の SQLite 実装 — `read_plan_fingerprint` 表 1 つだけを読み書きする。
//!
//! 行を書いて読み戻す試験は `tests/reference_surface_updater_contract.rs` にある — 行
//! ([`PlanFingerprintRow`]) は実行と intent の履歴からしか組めないので、実ストアの履歴を
//! 用意できる結合試験の側で見る。

use rusqlite::{Connection, OptionalExtension as _, Transaction, params};

use super::journal_reader_impl::corrupt_error;
use super::store_failure::SqliteResultExt;
use super::{CorruptCause, GlobalSeqNr, JournalReadError, PlanFingerprintDao, SourceStamp};
use crate::read_tables::PlanFingerprintRow;

/// 表の DDL (冪等)。主キーは (`execution_id`, `target_id`) から導いた代理キーで、自然キーの
/// 重複は UNIQUE 制約が止める。
const CREATE_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS read_plan_fingerprint (
    id TEXT PRIMARY KEY,
    execution_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    fingerprint TEXT,
    error TEXT,
    source_digest TEXT NOT NULL,
    as_of INTEGER NOT NULL,
    UNIQUE(execution_id,target_id)
);";

/// 行が名乗る出所。
const SELECT_STAMP: &str = "SELECT source_digest, as_of FROM read_plan_fingerprint WHERE id = ?1";

/// 主キーの行を消す (差し替えの前半)。
const DELETE: &str = "DELETE FROM read_plan_fingerprint WHERE id = ?1";

/// 行を足す。
const INSERT: &str = "INSERT INTO read_plan_fingerprint
     (id, execution_id, target_id, fingerprint, error, source_digest, as_of)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)";

/// `read_plan_fingerprint` 表の DAO の実装。状態を持たない。
#[derive(Debug, Clone, Copy, Default)]
pub struct PlanFingerprintDaoImpl;

impl PlanFingerprintDao for PlanFingerprintDaoImpl {
    fn create_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError> {
        transaction
            .execute_batch(CREATE_TABLE)
            .at_connection(transaction)
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
        row: &PlanFingerprintRow,
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
                    row.fingerprint(),
                    row.error(),
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
        PlanFingerprintDaoImpl
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
            PlanFingerprintDaoImpl
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
                "INSERT INTO read_plan_fingerprint VALUES ('a','e','t','f',NULL,'digest',7)",
                [],
            )
            .unwrap();
        assert_eq!(
            PlanFingerprintDaoImpl.find_stamp(&connection, "a").unwrap(),
            Some(SourceStamp::new("digest".to_string(), GlobalSeqNr::new(7)))
        );
        connection
            .execute("UPDATE read_plan_fingerprint SET as_of = -1", [])
            .unwrap();
        assert_eq!(
            PlanFingerprintDaoImpl
                .find_stamp(&connection, "a")
                .unwrap_err(),
            corrupt_error("a", None, CorruptCause::InvariantViolation)
        );
    }
}
