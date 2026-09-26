//! `ProjectionCheckpointDao` の SQLite 実装 — `amadeus_projection_checkpoint` 表 1 つだけを
//! 読み書きする。

use rusqlite::{Connection, OptionalExtension as _, Transaction, params};

use super::column_value::{integer, max_position, scan_position, table_exists};
use super::journal_reader_impl::corrupt_error;
use super::store_failure::SqliteResultExt;
use super::{
    CorruptCause, GlobalSeqNr, JournalAnchor, JournalReadError, ProjectionCheckpointDao,
    ProjectionCheckpointRow, ProjectionName,
};

/// 表の DDL (冪等)。本家の表とは名前を分ける — 同じ DB ファイルに同居させても本家の
/// スキーマ作成と衝突しない。
const CREATE_TABLE: &str = "CREATE TABLE IF NOT EXISTS amadeus_projection_checkpoint (
  projection      TEXT    PRIMARY KEY,
  last_global_seq INTEGER NOT NULL,
  anchor_aid      TEXT,
  anchor_seq_nr   INTEGER
)";

/// 表が在るか (`sqlite_master` を引くだけ — 書込ロックを取らない)。
const TABLE_EXISTS: &str = "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'amadeus_projection_checkpoint'";

/// 投影の行。
const SELECT: &str = "SELECT last_global_seq, anchor_aid, anchor_seq_nr
     FROM amadeus_projection_checkpoint WHERE projection = ?1";

/// 最も進んだ位置。
const SELECT_MAX_POSITION: &str = "SELECT MAX(last_global_seq) FROM amadeus_projection_checkpoint";

/// 行を保存する (未登録なら足す)。
const UPSERT: &str =
    "INSERT INTO amadeus_projection_checkpoint(projection, last_global_seq, anchor_aid, anchor_seq_nr)
     VALUES (?1, ?2, ?3, ?4)
     ON CONFLICT(projection) DO UPDATE SET
       last_global_seq = excluded.last_global_seq,
       anchor_aid = excluded.anchor_aid,
       anchor_seq_nr = excluded.anchor_seq_nr";

/// 集約に属さない値の識別子欄に置く印。
const NO_AGGREGATE: &str = "-";

/// `amadeus_projection_checkpoint` 表の DAO の実装。状態を持たない。
#[derive(Debug, Clone, Copy, Default)]
pub struct ProjectionCheckpointDaoImpl;

impl ProjectionCheckpointDao for ProjectionCheckpointDaoImpl {
    fn create_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError> {
        transaction
            .execute_batch(CREATE_TABLE)
            .at_connection(transaction)
    }

    fn table_exists(&self, connection: &Connection) -> Result<bool, JournalReadError> {
        table_exists(connection, TABLE_EXISTS)
    }

    fn find(
        &self,
        connection: &Connection,
        projection: &ProjectionName,
    ) -> Result<Option<ProjectionCheckpointRow>, JournalReadError> {
        let saved: Option<(i64, Option<String>, Option<i64>)> = connection
            .query_row(SELECT, params![projection.as_str()], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .optional()
            .at_connection(connection)?;
        let Some((position, anchor_aid, anchor_seq_nr)) = saved else {
            return Ok(None);
        };
        let position = u64::try_from(position)
            .map(GlobalSeqNr::new)
            .map_err(|_| corrupt_error(NO_AGGREGATE, None, CorruptCause::InvariantViolation))?;
        let anchor = match (anchor_aid, anchor_seq_nr) {
            (Some(aggregate_id), Some(seq_nr)) => {
                let seq_nr = usize::try_from(seq_nr).map_err(|_| {
                    corrupt_error(&aggregate_id, None, CorruptCause::InvariantViolation)
                })?;
                Some(JournalAnchor::new(aggregate_id, seq_nr))
            }
            _ => None,
        };
        Ok(Some(ProjectionCheckpointRow::new(
            projection.clone(),
            position,
            anchor,
        )))
    }

    fn find_max_position(
        &self,
        connection: &Connection,
    ) -> Result<Option<GlobalSeqNr>, JournalReadError> {
        max_position(connection, SELECT_MAX_POSITION)
    }

    fn save(
        &self,
        transaction: &mut Transaction<'_>,
        row: &ProjectionCheckpointRow,
    ) -> Result<(), JournalReadError> {
        let position = scan_position(row.position())?;
        let (anchor_aid, anchor_seq_nr) = match row.anchor() {
            Some(anchor) => (
                Some(anchor.aggregate_id()),
                Some(integer(anchor.seq_nr(), anchor.aggregate_id())?),
            ),
            None => (None, None),
        };
        transaction
            .execute(
                UPSERT,
                params![
                    row.projection().as_str(),
                    position,
                    anchor_aid,
                    anchor_seq_nr
                ],
            )
            .at_connection(transaction)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn projection(name: &str) -> ProjectionName {
        ProjectionName::parse(name).unwrap()
    }

    fn created() -> Connection {
        let mut connection = Connection::open_in_memory().unwrap();
        let mut transaction = connection.transaction().unwrap();
        ProjectionCheckpointDaoImpl
            .create_table(&mut transaction)
            .unwrap();
        transaction.commit().unwrap();
        connection
    }

    fn saved(connection: &mut Connection, row: &ProjectionCheckpointRow) {
        let mut transaction = connection.transaction().unwrap();
        ProjectionCheckpointDaoImpl
            .save(&mut transaction, row)
            .unwrap();
        transaction.commit().unwrap();
    }

    #[test]
    fn an_unsaved_projection_has_no_row() {
        let connection = created();
        assert!(
            ProjectionCheckpointDaoImpl
                .table_exists(&connection)
                .unwrap()
        );
        assert_eq!(
            ProjectionCheckpointDaoImpl
                .find(&connection, &projection("state-file"))
                .unwrap(),
            None
        );
        assert_eq!(
            ProjectionCheckpointDaoImpl
                .find_max_position(&connection)
                .unwrap(),
            None
        );
    }

    #[test]
    fn a_saved_row_reads_back_with_its_anchor_and_a_later_save_overwrites_it() {
        let mut connection = created();
        let first = ProjectionCheckpointRow::new(
            projection("state-file"),
            GlobalSeqNr::new(3),
            Some(JournalAnchor::new("intent-x".to_string(), 1)),
        );
        saved(&mut connection, &first);
        assert_eq!(
            ProjectionCheckpointDaoImpl
                .find(&connection, &projection("state-file"))
                .unwrap(),
            Some(first)
        );
        let second = ProjectionCheckpointRow::new(
            projection("state-file"),
            GlobalSeqNr::new(7),
            Some(JournalAnchor::new("intent-y".to_string(), 2)),
        );
        saved(&mut connection, &second);
        assert_eq!(
            ProjectionCheckpointDaoImpl
                .find(&connection, &projection("state-file"))
                .unwrap(),
            Some(second)
        );
        let zero = ProjectionCheckpointRow::new(projection("other"), GlobalSeqNr::ZERO, None);
        saved(&mut connection, &zero);
        assert_eq!(
            ProjectionCheckpointDaoImpl
                .find(&connection, &projection("other"))
                .unwrap(),
            Some(zero)
        );
        assert_eq!(
            ProjectionCheckpointDaoImpl
                .find_max_position(&connection)
                .unwrap(),
            Some(GlobalSeqNr::new(7)),
            "投影をまたいだ最大値"
        );
    }

    #[test]
    fn a_half_saved_anchor_reads_as_no_anchor() {
        let connection = created();
        connection
            .execute(
                "INSERT INTO amadeus_projection_checkpoint(projection, last_global_seq, anchor_aid)
                 VALUES ('state-file', 3, 'intent-x')",
                [],
            )
            .unwrap();
        let row = ProjectionCheckpointDaoImpl
            .find(&connection, &projection("state-file"))
            .unwrap()
            .unwrap();
        assert_eq!(row.anchor(), None);
    }

    #[test]
    fn negative_saved_values_are_corrupt() {
        let connection = created();
        connection
            .execute(
                "INSERT INTO amadeus_projection_checkpoint VALUES ('state-file', -1, NULL, NULL),
                 ('anchored', 3, 'intent-x', -5)",
                [],
            )
            .unwrap();
        assert_eq!(
            ProjectionCheckpointDaoImpl
                .find(&connection, &projection("state-file"))
                .unwrap_err(),
            corrupt_error(NO_AGGREGATE, None, CorruptCause::InvariantViolation)
        );
        assert_eq!(
            ProjectionCheckpointDaoImpl
                .find(&connection, &projection("anchored"))
                .unwrap_err(),
            corrupt_error("intent-x", None, CorruptCause::InvariantViolation)
        );
    }

    #[test]
    fn a_position_beyond_the_column_is_corrupt_rather_than_truncated() {
        let mut connection = created();
        let mut transaction = connection.transaction().unwrap();
        assert_eq!(
            ProjectionCheckpointDaoImpl
                .save(
                    &mut transaction,
                    &ProjectionCheckpointRow::new(
                        projection("state-file"),
                        GlobalSeqNr::new(u64::MAX),
                        None
                    )
                )
                .unwrap_err(),
            corrupt_error(NO_AGGREGATE, None, CorruptCause::InvariantViolation)
        );
    }
}
