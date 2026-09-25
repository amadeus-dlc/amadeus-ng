//! `WorkspaceDoctorProjectionCheckpointDao` の SQLite 実装 —
//! `workspace_doctor_projection_checkpoint` 表 1 つだけを読み書きする。

use rusqlite::{Connection, OptionalExtension as _, Transaction, params};

use super::journal_reader_impl::corrupt_error;
use super::store_failure::SqliteResultExt;
use super::{
    CorruptCause, GlobalSeqNr, JournalReadError, ProjectionName,
    WorkspaceDoctorProjectionCheckpointDao,
};

/// 表の DDL (冪等)。
const CREATE_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS workspace_doctor_projection_checkpoint (
  projection TEXT    PRIMARY KEY,
  last_seq   INTEGER NOT NULL
);";

/// 処理したシーケンス番号を引く。
const SELECT: &str =
    "SELECT last_seq FROM workspace_doctor_projection_checkpoint WHERE projection = ?1";

/// 処理したシーケンス番号を保存する (未登録なら足す)。
const UPSERT: &str = "INSERT INTO workspace_doctor_projection_checkpoint(projection, last_seq)
     VALUES (?1, ?2)
     ON CONFLICT(projection) DO UPDATE SET last_seq = excluded.last_seq";

/// `workspace_doctor_projection_checkpoint` 表の DAO の実装。状態を持たない。
#[derive(Debug, Clone, Copy, Default)]
pub struct WorkspaceDoctorProjectionCheckpointDaoImpl;

impl WorkspaceDoctorProjectionCheckpointDao for WorkspaceDoctorProjectionCheckpointDaoImpl {
    fn create_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError> {
        transaction
            .execute_batch(CREATE_TABLE)
            .at_connection(transaction)
    }

    fn find(
        &self,
        connection: &Connection,
        projection: &ProjectionName,
    ) -> Result<GlobalSeqNr, JournalReadError> {
        let saved: Option<i64> = connection
            .query_row(SELECT, [projection.as_str()], |row| row.get(0))
            .optional()
            .at_connection(connection)?;
        saved.map_or(Ok(GlobalSeqNr::ZERO), |value| {
            u64::try_from(value).map(GlobalSeqNr::new).map_err(|_| {
                corrupt_error(projection.as_str(), None, CorruptCause::InvariantViolation)
            })
        })
    }

    fn save(
        &self,
        transaction: &mut Transaction<'_>,
        projection: &ProjectionName,
        position: GlobalSeqNr,
    ) -> Result<(), JournalReadError> {
        let value = i64::try_from(position.to_u64()).map_err(|_| {
            corrupt_error(projection.as_str(), None, CorruptCause::InvariantViolation)
        })?;
        transaction
            .execute(UPSERT, params![projection.as_str(), value])
            .at_connection(transaction)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn projection() -> ProjectionName {
        ProjectionName::parse("workspace-doctor").unwrap()
    }

    fn saved(connection: &mut Connection, position: GlobalSeqNr) {
        let mut transaction = connection.transaction().unwrap();
        let dao = WorkspaceDoctorProjectionCheckpointDaoImpl;
        dao.create_table(&mut transaction).unwrap();
        dao.save(&mut transaction, &projection(), position).unwrap();
        transaction.commit().unwrap();
    }

    fn created(connection: &mut Connection) {
        let mut transaction = connection.transaction().unwrap();
        WorkspaceDoctorProjectionCheckpointDaoImpl
            .create_table(&mut transaction)
            .unwrap();
        transaction.commit().unwrap();
    }

    #[test]
    fn an_unsaved_projection_starts_from_zero() {
        let mut connection = Connection::open_in_memory().unwrap();
        created(&mut connection);
        assert_eq!(
            WorkspaceDoctorProjectionCheckpointDaoImpl
                .find(&connection, &projection())
                .unwrap(),
            GlobalSeqNr::ZERO
        );
    }

    #[test]
    fn a_saved_position_reads_back_and_a_later_save_overwrites_it() {
        let mut connection = Connection::open_in_memory().unwrap();
        saved(&mut connection, GlobalSeqNr::new(3));
        assert_eq!(
            WorkspaceDoctorProjectionCheckpointDaoImpl
                .find(&connection, &projection())
                .unwrap(),
            GlobalSeqNr::new(3)
        );
        saved(&mut connection, GlobalSeqNr::new(7));
        assert_eq!(
            WorkspaceDoctorProjectionCheckpointDaoImpl
                .find(&connection, &projection())
                .unwrap(),
            GlobalSeqNr::new(7)
        );
        let rows: i64 = connection
            .query_row(
                "SELECT count(*) FROM workspace_doctor_projection_checkpoint",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(rows, 1, "投影ごとに 1 行");
    }

    #[test]
    fn a_negative_saved_value_is_corrupt() {
        let mut connection = Connection::open_in_memory().unwrap();
        created(&mut connection);
        connection
            .execute(
                "INSERT INTO workspace_doctor_projection_checkpoint VALUES ('workspace-doctor', -1)",
                [],
            )
            .unwrap();
        assert_eq!(
            WorkspaceDoctorProjectionCheckpointDaoImpl
                .find(&connection, &projection())
                .unwrap_err(),
            corrupt_error("workspace-doctor", None, CorruptCause::InvariantViolation)
        );
    }

    #[test]
    fn a_position_beyond_the_column_is_corrupt_rather_than_truncated() {
        let mut connection = Connection::open_in_memory().unwrap();
        let mut transaction = connection.transaction().unwrap();
        let dao = WorkspaceDoctorProjectionCheckpointDaoImpl;
        dao.create_table(&mut transaction).unwrap();
        assert_eq!(
            dao.save(&mut transaction, &projection(), GlobalSeqNr::new(u64::MAX))
                .unwrap_err(),
            corrupt_error("workspace-doctor", None, CorruptCause::InvariantViolation)
        );
    }
}
