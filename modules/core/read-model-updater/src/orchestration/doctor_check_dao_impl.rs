//! `DoctorCheckDao` の SQLite 実装 — `read_doctor_check` 表 1 つだけを読み書きする。

use rusqlite::{Transaction, params};

use super::journal_reader_impl::corrupt_error;
use super::store_failure::SqliteResultExt;
use super::{CorruptCause, DoctorCheckDao, DoctorCheckRow, JournalReadError};

/// 表と索引の DDL (冪等)。FK 列 `report_id` で引くので索引を張り、自然キー
/// (`report_id` × `position`) の重複は UNIQUE 索引で止める。
const CREATE_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS read_doctor_check (
  id        TEXT    PRIMARY KEY,
  report_id TEXT    NOT NULL,
  position  INTEGER NOT NULL,
  check_id  TEXT    NOT NULL,
  passed    INTEGER NOT NULL,
  label     TEXT    NOT NULL,
  fix       TEXT
);
CREATE INDEX IF NOT EXISTS read_doctor_check_report_idx ON read_doctor_check (report_id);
CREATE UNIQUE INDEX IF NOT EXISTS read_doctor_check_order_idx
  ON read_doctor_check (report_id, position);";

/// 1 つの報告の行を消す。
const DELETE_FOR_REPORT: &str = "DELETE FROM read_doctor_check WHERE report_id = ?1";

/// 行を足す。
const INSERT: &str =
    "INSERT INTO read_doctor_check(id, report_id, position, check_id, passed, label, fix)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)";

/// `read_doctor_check` 表の DAO の実装。状態を持たない。
#[derive(Debug, Clone, Copy, Default)]
pub struct DoctorCheckDaoImpl;

impl DoctorCheckDao for DoctorCheckDaoImpl {
    fn create_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError> {
        transaction
            .execute_batch(CREATE_TABLE)
            .at_connection(transaction)
    }

    fn replace_for_report(
        &self,
        transaction: &mut Transaction<'_>,
        report_id: &str,
        rows: &[DoctorCheckRow],
    ) -> Result<(), JournalReadError> {
        transaction
            .execute(DELETE_FOR_REPORT, [report_id])
            .at_connection(transaction)?;
        for row in rows {
            let position = i64::try_from(row.position())
                .map_err(|_| corrupt_error(report_id, None, CorruptCause::InvariantViolation))?;
            transaction
                .execute(
                    INSERT,
                    params![
                        row.id(),
                        row.report_id(),
                        position,
                        row.check_id(),
                        i64::from(row.is_passed()),
                        row.label(),
                        row.fix(),
                    ],
                )
                .at_connection(transaction)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // テストは固定長フィクスチャの添字参照を許容 (オーナー規約)。
    #![allow(clippy::indexing_slicing)]

    use super::*;
    use rusqlite::Connection;

    type ReadBack = (String, i64, String, i64, String, Option<String>);

    fn check(report_id: &str, position: usize, passed: bool) -> DoctorCheckRow {
        DoctorCheckRow::new(
            report_id.to_string(),
            position,
            format!("D{position}"),
            passed,
            format!("label {position}"),
            (!passed).then(|| format!("fix {position}")),
        )
    }

    fn replaced(connection: &mut Connection, report_id: &str, rows: &[DoctorCheckRow]) {
        let mut transaction = connection.transaction().unwrap();
        DoctorCheckDaoImpl.create_table(&mut transaction).unwrap();
        DoctorCheckDaoImpl
            .replace_for_report(&mut transaction, report_id, rows)
            .unwrap();
        transaction.commit().unwrap();
    }

    fn read_back(connection: &Connection, report_id: &str) -> Vec<ReadBack> {
        let mut statement = connection
            .prepare(
                "SELECT report_id, position, check_id, passed, label, fix FROM read_doctor_check
                 WHERE report_id = ?1 ORDER BY position",
            )
            .unwrap();
        statement
            .query_map([report_id], |row| {
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
    fn replaced_rows_read_back_in_display_order() {
        let mut connection = Connection::open_in_memory().unwrap();
        replaced(
            &mut connection,
            "r",
            &[check("r", 0, true), check("r", 1, false)],
        );
        assert_eq!(
            read_back(&connection, "r"),
            [
                (
                    "r".to_string(),
                    0,
                    "D0".to_string(),
                    1,
                    "label 0".to_string(),
                    None
                ),
                (
                    "r".to_string(),
                    1,
                    "D1".to_string(),
                    0,
                    "label 1".to_string(),
                    Some("fix 1".to_string())
                ),
            ]
        );
    }

    #[test]
    fn replacing_drops_the_previous_rows_of_that_report_only() {
        let mut connection = Connection::open_in_memory().unwrap();
        replaced(
            &mut connection,
            "r",
            &[check("r", 0, false), check("r", 1, false)],
        );
        replaced(&mut connection, "other", &[check("other", 0, true)]);
        replaced(&mut connection, "r", &[check("r", 0, true)]);
        let rows = read_back(&connection, "r");
        assert_eq!(rows.len(), 1, "前回の 2 行目は残らない");
        assert_eq!(rows[0].3, 1, "前回の失敗行は置き換わった");
        assert_eq!(
            read_back(&connection, "other").len(),
            1,
            "別の報告には触れない"
        );
    }

    #[test]
    fn the_primary_key_is_derived_from_the_report_and_the_position() {
        let mut connection = Connection::open_in_memory().unwrap();
        let row = check("r", 0, true);
        replaced(&mut connection, "r", std::slice::from_ref(&row));
        let id: String = connection
            .query_row("SELECT id FROM read_doctor_check", [], |found| found.get(0))
            .unwrap();
        assert_eq!(id, crate::read_tables::doctor_check("r", 0));
        assert_eq!(id, row.id());
    }

    #[test]
    fn replacing_with_no_rows_empties_the_report() {
        let mut connection = Connection::open_in_memory().unwrap();
        replaced(&mut connection, "r", &[check("r", 0, true)]);
        replaced(&mut connection, "r", &[]);
        assert!(read_back(&connection, "r").is_empty());
    }
}
