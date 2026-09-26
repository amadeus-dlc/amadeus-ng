//! `PipelineProgressDao` の SQLite 実装 — `read_pipeline_progress` 表 1 つだけを読み書きする。
//!
//! 実際の投影 ([`crate::read_tables::PipelineTables`]) が実ストアの履歴から組んだ行で書いて
//! 読み戻す試験は `tests/reference_surface_updater_contract.rs` にある。ここでは列の往復・
//! 実行ごとの差し替え・値域の失敗を、手で組んだ行で見る。

use rusqlite::{Connection, OptionalExtension as _, Transaction, params};

use super::journal_reader_impl::corrupt_error;
use super::store_failure::SqliteResultExt;
use super::{
    CorruptCause, GlobalSeqNr, JournalReadError, PipelineProgressDao, PipelineProgressRow,
    SourceStamp,
};

/// 表の DDL (冪等)。主キーは (`execution_id`, `stage`, `single`) から導いた代理キーで、自然キーの
/// 重複は UNIQUE 制約が止める。
const CREATE_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS read_pipeline_progress (
    id TEXT PRIMARY KEY,
    execution_id TEXT NOT NULL,
    stage TEXT NOT NULL,
    single INTEGER NOT NULL,
    completed TEXT NOT NULL,
    source_digest TEXT NOT NULL,
    event_position INTEGER NOT NULL,
    UNIQUE(execution_id, stage, single)
);";

/// 実行の行が名乗る出所 (同じ実行の行はどれも同じ値なので 1 行だけ読む)。
const SELECT_STAMP: &str = "SELECT source_digest, event_position FROM read_pipeline_progress
     WHERE execution_id = ?1 LIMIT 1";

/// 実行の行を消す (差し替えの前半)。
const DELETE_FOR_EXECUTION: &str = "DELETE FROM read_pipeline_progress WHERE execution_id = ?1";

/// 行を足す。
const INSERT: &str = "INSERT INTO read_pipeline_progress
     (id, execution_id, stage, single, completed, source_digest, event_position)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)";

/// 表を落とす (索引も一緒に落ちる)。
const DROP_TABLE: &str = "DROP TABLE IF EXISTS read_pipeline_progress";

/// 表が在るか (`sqlite_master` を引くだけ — 書込ロックを取らない)。
const TABLE_EXISTS: &str =
    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'read_pipeline_progress'";

/// `read_pipeline_progress` 表の DAO の実装。状態を持たない。
#[derive(Debug, Clone, Copy, Default)]
pub struct PipelineProgressDaoImpl;

impl PipelineProgressDao for PipelineProgressDaoImpl {
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
        execution_id: &str,
    ) -> Result<Option<SourceStamp>, JournalReadError> {
        let saved: Option<(String, i64)> = connection
            .query_row(SELECT_STAMP, [execution_id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .optional()
            .at_connection(connection)?;
        saved
            .map(|(source_digest, position)| {
                u64::try_from(position)
                    .map(|position| SourceStamp::new(source_digest, GlobalSeqNr::new(position)))
                    .map_err(|_| {
                        corrupt_error(execution_id, None, CorruptCause::InvariantViolation)
                    })
            })
            .transpose()
    }

    fn replace_for_execution(
        &self,
        transaction: &mut Transaction<'_>,
        execution_id: &str,
        rows: &[PipelineProgressRow],
    ) -> Result<(), JournalReadError> {
        transaction
            .execute(DELETE_FOR_EXECUTION, [execution_id])
            .at_connection(transaction)?;
        for row in rows {
            let position = i64::try_from(row.event_position().to_u64())
                .map_err(|_| corrupt_error(row.id(), None, CorruptCause::InvariantViolation))?;
            transaction
                .execute(
                    INSERT,
                    params![
                        row.id(),
                        row.execution_id(),
                        row.stage(),
                        row.is_single(),
                        row.completed(),
                        row.source_digest(),
                        position,
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

    fn row(execution_id: &str, stage: &str, single: bool, position: u64) -> PipelineProgressRow {
        PipelineProgressRow::new(
            format!("pipeline:{execution_id}:{stage}:{single}"),
            execution_id.to_string(),
            stage.to_string(),
            single,
            "[\"aidlc-developer-agent\"]".to_string(),
            format!("digest-{position}"),
            GlobalSeqNr::new(position),
        )
    }

    fn created() -> Connection {
        let mut connection = Connection::open_in_memory().unwrap();
        let mut transaction = connection.transaction().unwrap();
        PipelineProgressDaoImpl
            .create_table(&mut transaction)
            .unwrap();
        transaction.commit().unwrap();
        connection
    }

    fn replaced(connection: &mut Connection, execution_id: &str, rows: &[PipelineProgressRow]) {
        let mut transaction = connection.transaction().unwrap();
        PipelineProgressDaoImpl
            .replace_for_execution(&mut transaction, execution_id, rows)
            .unwrap();
        transaction.commit().unwrap();
    }

    type Columns = (String, String, String, bool, String, String, i64);

    fn read_back(connection: &Connection) -> Vec<Columns> {
        let mut statement = connection
            .prepare(
                "SELECT id, execution_id, stage, single, completed, source_digest, event_position
                 FROM read_pipeline_progress ORDER BY id",
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
                    row.get(6)?,
                ))
            })
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    fn columns(row: &PipelineProgressRow) -> Columns {
        (
            row.id().to_string(),
            row.execution_id().to_string(),
            row.stage().to_string(),
            row.is_single(),
            row.completed().to_string(),
            row.source_digest().to_string(),
            i64::try_from(row.event_position().to_u64()).unwrap(),
        )
    }

    #[test]
    fn the_table_exists_only_after_it_is_created_and_creating_twice_is_harmless() {
        let mut connection = Connection::open_in_memory().unwrap();
        assert!(!PipelineProgressDaoImpl.table_exists(&connection).unwrap());
        for _ in 0..2 {
            let mut transaction = connection.transaction().unwrap();
            PipelineProgressDaoImpl
                .create_table(&mut transaction)
                .unwrap();
            transaction.commit().unwrap();
        }
        assert!(PipelineProgressDaoImpl.table_exists(&connection).unwrap());
    }

    #[test]
    fn the_replaced_rows_read_back_column_for_column_with_their_stamp() {
        let mut connection = created();
        let rows = [
            row("e1", "reverse-engineering", false, 7),
            row("e1", "reverse-engineering", true, 7),
        ];
        replaced(&mut connection, "e1", &rows);
        assert_eq!(
            read_back(&connection),
            rows.iter().map(columns).collect::<Vec<_>>()
        );
        assert_eq!(
            PipelineProgressDaoImpl
                .find_stamp(&connection, "e1")
                .unwrap(),
            Some(SourceStamp::new(
                "digest-7".to_string(),
                GlobalSeqNr::new(7)
            ))
        );
    }

    #[test]
    fn an_execution_without_rows_has_no_stamp() {
        let connection = created();
        assert_eq!(
            PipelineProgressDaoImpl
                .find_stamp(&connection, "e1")
                .unwrap(),
            None
        );
    }

    #[test]
    fn a_replacement_touches_only_the_rows_of_its_execution() {
        let mut connection = created();
        let other = row("e2", "reverse-engineering", false, 3);
        replaced(&mut connection, "e2", std::slice::from_ref(&other));
        replaced(
            &mut connection,
            "e1",
            &[row("e1", "reverse-engineering", false, 5)],
        );
        let newer = row("e1", "ci-pipeline", true, 9);
        replaced(&mut connection, "e1", std::slice::from_ref(&newer));
        assert_eq!(
            read_back(&connection),
            vec![columns(&newer), columns(&other)]
        );

        // 空の行で差し替えれば、その実行の行は消える (ほかの実行は残る)。
        replaced(&mut connection, "e1", &[]);
        assert_eq!(read_back(&connection), vec![columns(&other)]);
    }

    #[test]
    fn a_negative_saved_position_is_corrupt() {
        let mut connection = created();
        replaced(
            &mut connection,
            "e1",
            &[row("e1", "reverse-engineering", false, 1)],
        );
        connection
            .execute("UPDATE read_pipeline_progress SET event_position = -1", [])
            .unwrap();
        assert_eq!(
            PipelineProgressDaoImpl
                .find_stamp(&connection, "e1")
                .unwrap_err(),
            corrupt_error("e1", None, CorruptCause::InvariantViolation)
        );
    }

    #[test]
    fn a_position_beyond_the_column_is_corrupt_rather_than_truncated() {
        let mut connection = created();
        let mut transaction = connection.transaction().unwrap();
        let huge = row("e1", "reverse-engineering", false, u64::MAX);
        assert_eq!(
            PipelineProgressDaoImpl.replace_for_execution(
                &mut transaction,
                "e1",
                std::slice::from_ref(&huge)
            ),
            Err(corrupt_error(
                huge.id(),
                None,
                CorruptCause::InvariantViolation
            ))
        );
    }
}
