//! `DoctorReportDao` の SQLite 実装 — `read_doctor_report` 表 1 つだけを読み書きする。

use rusqlite::{Transaction, params};

use super::journal_reader_impl::corrupt_error;
use super::store_failure::SqliteResultExt;
use super::{CorruptCause, DoctorReportDao, DoctorReportRow, JournalReadError};

/// 表と索引の DDL (冪等)。自然キー `target` の重複は UNIQUE 索引で止める。
const CREATE_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS read_doctor_report (
  id        TEXT    PRIMARY KEY,
  target    TEXT    NOT NULL,
  passed    INTEGER NOT NULL,
  failed    INTEGER NOT NULL,
  exit_code INTEGER NOT NULL,
  seq_nr    INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS read_doctor_report_target_idx ON read_doctor_report (target);";

/// 主キーで差し替える (無ければ足す)。
const UPSERT: &str = "INSERT INTO read_doctor_report(id, target, passed, failed, exit_code, seq_nr)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6)
     ON CONFLICT(id) DO UPDATE SET
       target = excluded.target,
       passed = excluded.passed,
       failed = excluded.failed,
       exit_code = excluded.exit_code,
       seq_nr = excluded.seq_nr";

/// `read_doctor_report` 表の DAO の実装。状態を持たない。
#[derive(Debug, Clone, Copy, Default)]
pub struct DoctorReportDaoImpl;

impl DoctorReportDao for DoctorReportDaoImpl {
    fn create_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError> {
        transaction
            .execute_batch(CREATE_TABLE)
            .at_connection(transaction)
    }

    fn save(
        &self,
        transaction: &mut Transaction<'_>,
        row: &DoctorReportRow,
    ) -> Result<(), JournalReadError> {
        let integer = |value: u64| {
            i64::try_from(value)
                .map_err(|_| corrupt_error(row.id(), None, CorruptCause::InvariantViolation))
        };
        let seq_nr = i64::try_from(row.seq_nr())
            .map_err(|_| corrupt_error(row.id(), None, CorruptCause::InvariantViolation))?;
        transaction
            .execute(
                UPSERT,
                params![
                    row.id(),
                    row.target(),
                    integer(row.passed())?,
                    integer(row.failed())?,
                    i64::from(row.exit_code()),
                    seq_nr,
                ],
            )
            .at_connection(transaction)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // テストは固定長フィクスチャの添字参照を許容 (オーナー規約)。
    #![allow(clippy::indexing_slicing)]

    use super::*;
    use rusqlite::Connection;

    fn row(id: &str, target: &str, failed: u64, seq_nr: usize) -> DoctorReportRow {
        DoctorReportRow::new(id.to_string(), target.to_string(), 3, failed, 1, seq_nr)
    }

    fn written(connection: &mut Connection, rows: &[DoctorReportRow]) {
        let mut transaction = connection.transaction().unwrap();
        DoctorReportDaoImpl.create_table(&mut transaction).unwrap();
        for row in rows {
            DoctorReportDaoImpl.save(&mut transaction, row).unwrap();
        }
        transaction.commit().unwrap();
    }

    fn read_back(connection: &Connection) -> Vec<(String, String, i64, i64, i64, i64)> {
        let mut statement = connection
            .prepare(
                "SELECT id, target, passed, failed, exit_code, seq_nr FROM read_doctor_report ORDER BY id",
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
    fn a_saved_row_reads_back_column_for_column() {
        let mut connection = Connection::open_in_memory().unwrap();
        written(&mut connection, &[row("a", "spaces/default/intents", 1, 2)]);
        assert_eq!(
            read_back(&connection),
            [(
                "a".to_string(),
                "spaces/default/intents".to_string(),
                3,
                1,
                1,
                2
            )]
        );
    }

    #[test]
    fn saving_the_same_id_again_replaces_the_row_instead_of_adding_one() {
        let mut connection = Connection::open_in_memory().unwrap();
        written(&mut connection, &[row("a", "spaces/default/intents", 1, 1)]);
        written(&mut connection, &[row("a", "spaces/default/intents", 0, 2)]);
        let rows = read_back(&connection);
        assert_eq!(rows.len(), 1);
        assert_eq!((rows[0].3, rows[0].5), (0, 2));
    }

    #[test]
    fn two_reports_for_the_same_target_are_refused_by_the_unique_index() {
        let mut connection = Connection::open_in_memory().unwrap();
        written(&mut connection, &[row("a", "spaces/default/intents", 0, 1)]);
        let mut transaction = connection.transaction().unwrap();
        let error = DoctorReportDaoImpl
            .save(&mut transaction, &row("b", "spaces/default/intents", 0, 1))
            .unwrap_err();
        assert!(
            matches!(error, JournalReadError::Io { .. }),
            "実際: {error}"
        );
    }

    #[test]
    fn creating_the_table_twice_is_harmless() {
        let mut connection = Connection::open_in_memory().unwrap();
        written(&mut connection, &[]);
        written(&mut connection, &[row("a", "spaces/default/intents", 0, 1)]);
        assert_eq!(read_back(&connection).len(), 1);
    }

    #[test]
    fn a_count_that_does_not_fit_the_column_is_corrupt_rather_than_truncated() {
        let mut connection = Connection::open_in_memory().unwrap();
        let mut transaction = connection.transaction().unwrap();
        DoctorReportDaoImpl.create_table(&mut transaction).unwrap();
        let huge = DoctorReportRow::new("a".to_string(), "t".to_string(), u64::MAX, 0, 0, 1);
        let error = DoctorReportDaoImpl
            .save(&mut transaction, &huge)
            .unwrap_err();
        assert_eq!(
            error,
            corrupt_error("a", None, CorruptCause::InvariantViolation)
        );
    }
}
