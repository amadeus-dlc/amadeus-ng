//! `TestingContractDao` の SQLite 実装 — `read_testing_contract` 表 1 つだけを読み書きする。

use rusqlite::{Connection, OptionalExtension as _, Transaction, params};

use super::journal_reader_impl::corrupt_error;
use super::store_failure::SqliteResultExt;
use super::{CorruptCause, GlobalSeqNr, JournalReadError, SourceStamp, TestingContractDao};
use crate::read_tables::TestingTables;

/// 表の DDL (冪等)。主キーは依頼 ID (依頼前の既定行は `bare-space`)。
const CREATE_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS read_testing_contract (
    id TEXT PRIMARY KEY,
    contract TEXT,
    rendered TEXT,
    error TEXT,
    source_digest TEXT NOT NULL,
    as_of INTEGER NOT NULL
);";

/// 行が名乗る出所。
const SELECT_STAMP: &str = "SELECT source_digest, as_of FROM read_testing_contract WHERE id = ?1";

/// 全行を消す (全差し替えの前半)。
const DELETE_ALL: &str = "DELETE FROM read_testing_contract";

/// 行を足す。
const INSERT: &str = "INSERT INTO read_testing_contract
     (id, contract, rendered, error, source_digest, as_of)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6)";

/// `read_testing_contract` 表の DAO の実装。状態を持たない。
#[derive(Debug, Clone, Copy, Default)]
pub struct TestingContractDaoImpl;

impl TestingContractDao for TestingContractDaoImpl {
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

    fn replace(
        &self,
        transaction: &mut Transaction<'_>,
        tables: &TestingTables,
    ) -> Result<(), JournalReadError> {
        let as_of = i64::try_from(tables.as_of().to_u64()).map_err(|_| {
            corrupt_error(
                tables.source_digest(),
                None,
                CorruptCause::InvariantViolation,
            )
        })?;
        transaction
            .execute(DELETE_ALL, [])
            .at_connection(transaction)?;
        for row in tables.rows() {
            transaction
                .execute(
                    INSERT,
                    params![
                        row.id(),
                        row.contract(),
                        row.rendered(),
                        row.error(),
                        tables.source_digest(),
                        as_of,
                    ],
                )
                .at_connection(transaction)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::JournalBatch;
    use core_command_domain::orchestration::TestingSections;

    fn tables(org: &str, scanned_to: Option<u64>) -> TestingTables {
        TestingTables::project(
            &JournalBatch::new(
                Vec::new(),
                Vec::new(),
                Vec::new(),
                scanned_to.map(GlobalSeqNr::new),
            ),
            &TestingSections::from_documents(org, "", ""),
        )
    }

    fn written(connection: &mut Connection, tables: &TestingTables) {
        let mut transaction = connection.transaction().unwrap();
        TestingContractDaoImpl
            .create_table(&mut transaction)
            .unwrap();
        TestingContractDaoImpl
            .replace(&mut transaction, tables)
            .unwrap();
        transaction.commit().unwrap();
    }

    type Columns = (
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
        i64,
    );

    fn read_back(connection: &Connection) -> Vec<Columns> {
        let mut statement = connection
            .prepare(
                "SELECT id, contract, rendered, error, source_digest, as_of
                 FROM read_testing_contract ORDER BY id",
            )
            .unwrap();
        statement
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            })
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    #[test]
    fn the_replaced_rows_read_back_column_for_column_with_their_stamp() {
        let mut connection = Connection::open_in_memory().unwrap();
        let tables = tables("## Testing Posture\n\n- Methodology: tdd\n", Some(4));
        written(&mut connection, &tables);
        let expected: Vec<Columns> = tables
            .rows()
            .iter()
            .map(|row| {
                (
                    row.id().to_string(),
                    row.contract().map(str::to_string),
                    row.rendered().map(str::to_string),
                    row.error().map(str::to_string),
                    tables.source_digest().to_string(),
                    4,
                )
            })
            .collect();
        assert_eq!(read_back(&connection), expected);
        assert_eq!(
            TestingContractDaoImpl
                .find_stamp(&connection, "bare-space")
                .unwrap(),
            Some(SourceStamp::new(
                tables.source_digest().to_string(),
                GlobalSeqNr::new(4)
            ))
        );
    }

    #[test]
    fn a_missing_row_has_no_stamp() {
        let mut connection = Connection::open_in_memory().unwrap();
        written(&mut connection, &tables("", None));
        assert_eq!(
            TestingContractDaoImpl
                .find_stamp(&connection, "no-such-intent")
                .unwrap(),
            None
        );
    }

    #[test]
    fn a_second_replacement_leaves_no_row_of_the_first() {
        let mut connection = Connection::open_in_memory().unwrap();
        let first = tables("", Some(1));
        let second = tables("## Testing Posture\n\n- Methodology: tdd\n", Some(2));
        written(&mut connection, &first);
        written(&mut connection, &second);
        let rows = read_back(&connection);
        assert_eq!(rows.len(), second.rows().len());
        assert!(
            rows.iter()
                .all(|row| row.4 == second.source_digest() && row.5 == 2)
        );
    }

    #[test]
    fn a_negative_saved_position_is_corrupt() {
        let mut connection = Connection::open_in_memory().unwrap();
        written(&mut connection, &tables("", None));
        connection
            .execute("UPDATE read_testing_contract SET as_of = -1", [])
            .unwrap();
        assert_eq!(
            TestingContractDaoImpl
                .find_stamp(&connection, "bare-space")
                .unwrap_err(),
            corrupt_error("bare-space", None, CorruptCause::InvariantViolation)
        );
    }

    #[test]
    fn a_position_beyond_the_column_is_corrupt_rather_than_truncated() {
        let mut connection = Connection::open_in_memory().unwrap();
        let mut transaction = connection.transaction().unwrap();
        TestingContractDaoImpl
            .create_table(&mut transaction)
            .unwrap();
        let huge = tables("", Some(u64::MAX));
        assert!(matches!(
            TestingContractDaoImpl.replace(&mut transaction, &huge),
            Err(JournalReadError::Corrupt { .. })
        ));
    }
}
