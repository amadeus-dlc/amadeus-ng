//! `ReadModelHeadDao` の SQLite 実装 — `amadeus_read_model_head` 表 1 つだけを読み書きする。

use rusqlite::{Connection, OptionalExtension as _, Transaction, params};

use super::column_value::table_exists;
use super::store_failure::SqliteResultExt;
use super::{JournalReadError, ReadModelHeadDao, ReadModelHeadRow};

/// 表の DDL (冪等)。1 行だけの表 (`singleton = 1`) で、値の域は表の制約が守る。
const CREATE_TABLE: &str="CREATE TABLE IF NOT EXISTS amadeus_read_model_head (
 singleton INTEGER PRIMARY KEY CHECK(singleton=1), position INTEGER NOT NULL CHECK(position>=0),
 generation INTEGER NOT NULL CHECK(generation>0), revision TEXT NOT NULL, content_digest TEXT NOT NULL,
 verified INTEGER NOT NULL CHECK(verified IN (0,1)))";

/// 表が在るか (`sqlite_master` を引くだけ — 書込ロックを取らない)。
const TABLE_EXISTS: &str =
    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'amadeus_read_model_head'";

/// 記録の行。
const SELECT: &str = "SELECT position,generation,revision,content_digest,verified FROM amadeus_read_model_head WHERE singleton=1";

/// 記録の行を保存する (無ければ足す)。
const UPSERT: &str = "INSERT INTO amadeus_read_model_head VALUES (1,?1,?2,?3,?4,?5) ON CONFLICT(singleton) DO UPDATE SET position=excluded.position,generation=excluded.generation,revision=excluded.revision,content_digest=excluded.content_digest,verified=excluded.verified";

/// `amadeus_read_model_head` 表の DAO の実装。状態を持たない。
#[derive(Debug, Clone, Copy, Default)]
pub struct ReadModelHeadDaoImpl;

impl ReadModelHeadDao for ReadModelHeadDaoImpl {
    fn create_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError> {
        transaction
            .execute_batch(CREATE_TABLE)
            .at_connection(transaction)
    }

    fn table_exists(&self, connection: &Connection) -> Result<bool, JournalReadError> {
        table_exists(connection, TABLE_EXISTS)
    }

    fn find(&self, connection: &Connection) -> Result<Option<ReadModelHeadRow>, JournalReadError> {
        connection
            .query_row(SELECT, [], |row| {
                Ok(ReadModelHeadRow::new(
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })
            .optional()
            .at_connection(connection)
    }

    fn save(
        &self,
        transaction: &mut Transaction<'_>,
        row: &ReadModelHeadRow,
    ) -> Result<(), JournalReadError> {
        transaction
            .execute(
                UPSERT,
                params![
                    row.position(),
                    row.generation(),
                    row.revision(),
                    row.content_digest(),
                    row.is_verified()
                ],
            )
            .at_connection(transaction)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn created() -> Connection {
        let mut connection = Connection::open_in_memory().unwrap();
        let mut transaction = connection.transaction().unwrap();
        ReadModelHeadDaoImpl.create_table(&mut transaction).unwrap();
        transaction.commit().unwrap();
        connection
    }

    fn saved(connection: &mut Connection, row: &ReadModelHeadRow) {
        let mut transaction = connection.transaction().unwrap();
        ReadModelHeadDaoImpl.save(&mut transaction, row).unwrap();
        transaction.commit().unwrap();
    }

    #[test]
    fn the_record_is_absent_until_saved_and_a_later_save_overwrites_every_column() {
        let mut connection = created();
        assert!(ReadModelHeadDaoImpl.table_exists(&connection).unwrap());
        assert_eq!(ReadModelHeadDaoImpl.find(&connection).unwrap(), None);
        let first = ReadModelHeadRow::new(0, 1, "rev-a".to_string(), String::new(), false);
        saved(&mut connection, &first);
        assert_eq!(ReadModelHeadDaoImpl.find(&connection).unwrap(), Some(first));
        let second = ReadModelHeadRow::new(5, 2, "rev-b".to_string(), "digest".to_string(), true);
        saved(&mut connection, &second);
        assert_eq!(
            ReadModelHeadDaoImpl.find(&connection).unwrap(),
            Some(second)
        );
        let rows: i64 = connection
            .query_row("SELECT COUNT(*) FROM amadeus_read_model_head", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(rows, 1, "1 行だけの表");
    }

    #[test]
    fn a_damaged_record_is_read_as_it_was_stored() {
        // 壊れた記録を見分けるのは更新器である。DAO は読めた値をそのまま返す。
        let mut connection = created();
        saved(
            &mut connection,
            &ReadModelHeadRow::new(0, 1, "rev".to_string(), String::new(), true),
        );
        connection
            .execute_batch(
                "PRAGMA ignore_check_constraints=ON;
                 UPDATE amadeus_read_model_head SET position=-1, generation=0;",
            )
            .unwrap();
        assert_eq!(
            ReadModelHeadDaoImpl.find(&connection).unwrap(),
            Some(ReadModelHeadRow::new(
                -1,
                0,
                "rev".to_string(),
                String::new(),
                true
            ))
        );
    }

    #[test]
    fn the_table_constraints_refuse_a_negative_position() {
        let mut connection = created();
        let mut transaction = connection.transaction().unwrap();
        assert!(matches!(
            ReadModelHeadDaoImpl.save(
                &mut transaction,
                &ReadModelHeadRow::new(-1, 1, "rev".to_string(), String::new(), true)
            ),
            Err(JournalReadError::Io { .. })
        ));
    }
}
