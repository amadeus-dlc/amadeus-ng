//! `SteeringPartDao` の SQLite 実装 — `read_steering_part` 表 1 つだけを読み書きする。

use rusqlite::{Connection, Transaction, params};

use super::journal_reader_impl::corrupt_error;
use super::store_failure::SqliteResultExt;
use super::{CorruptCause, JournalReadError, SteeringPartDao, SteeringPartRow};

/// 表と索引の DDL (冪等)。
///
/// 主キーは自然キー (`phase`, `part_index`) から導いた代理キー `id` で、自然キーの重複は
/// UNIQUE 索引が止める。クエリ側が FK 列 `steering_plan_id` と部の番号で引くので
/// セカンダリ索引を張る。`FOREIGN KEY` 句は書かない (`read_tables` の表の形の裁定)。
const CREATE_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS read_steering_part (
  id               TEXT    PRIMARY KEY,
  steering_plan_id TEXT    NOT NULL,
  phase            TEXT    NOT NULL,
  part_index       INTEGER NOT NULL,
  rules_content    TEXT    NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS read_steering_part_key
  ON read_steering_part(phase, part_index);
CREATE INDEX IF NOT EXISTS read_steering_part_plan
  ON read_steering_part(steering_plan_id, part_index);";

/// 全行を消す (全差し替えの前半)。
const DELETE_ALL: &str = "DELETE FROM read_steering_part";

/// 行を足す。
const INSERT: &str = "INSERT INTO read_steering_part
     (id, steering_plan_id, phase, part_index, rules_content)
     VALUES (?1, ?2, ?3, ?4, ?5)";

/// 表が在るか (`sqlite_master` を引くだけ — 書込ロックを取らない)。
const TABLE_EXISTS: &str =
    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'read_steering_part'";

/// `read_steering_part` 表の DAO の実装。状態を持たない。
#[derive(Debug, Clone, Copy, Default)]
pub struct SteeringPartDaoImpl;

impl SteeringPartDao for SteeringPartDaoImpl {
    fn create_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError> {
        transaction
            .execute_batch(CREATE_TABLE)
            .at_connection(transaction)
    }

    fn table_exists(&self, connection: &Connection) -> Result<bool, JournalReadError> {
        let found: i64 = connection
            .query_row(TABLE_EXISTS, [], |row| row.get(0))
            .at_connection(connection)?;
        Ok(found > 0)
    }

    fn replace(
        &self,
        transaction: &mut Transaction<'_>,
        rows: &[SteeringPartRow],
    ) -> Result<(), JournalReadError> {
        transaction
            .execute(DELETE_ALL, [])
            .at_connection(transaction)?;
        for row in rows {
            let part_index = i64::try_from(row.part_index())
                .map_err(|_| corrupt_error(row.id(), None, CorruptCause::InvariantViolation))?;
            transaction
                .execute(
                    INSERT,
                    params![
                        row.id(),
                        row.steering_plan_id(),
                        row.phase(),
                        part_index,
                        row.rules_content(),
                    ],
                )
                .at_connection(transaction)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use rusqlite::Connection;

    use super::*;
    use crate::read_tables::{MemoryRules, RuleContent, SteeringTables};

    fn tables(text: &str) -> SteeringTables {
        SteeringTables::pack(&MemoryRules::new(
            vec![RuleContent::new("org.md".to_string(), text.to_string())],
            BTreeMap::new(),
        ))
        .unwrap()
    }

    fn written(connection: &mut Connection, rows: &[SteeringPartRow]) {
        let mut transaction = connection.transaction().unwrap();
        SteeringPartDaoImpl.create_table(&mut transaction).unwrap();
        SteeringPartDaoImpl.replace(&mut transaction, rows).unwrap();
        transaction.commit().unwrap();
    }

    fn read_back(connection: &Connection) -> Vec<(String, String, String, i64, String)> {
        let mut statement = connection
            .prepare(
                "SELECT id, steering_plan_id, phase, part_index, rules_content
                 FROM read_steering_part ORDER BY phase, part_index",
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
                ))
            })
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    #[test]
    fn the_replaced_rows_read_back_column_for_column() {
        let mut connection = Connection::open_in_memory().unwrap();
        let tables = tables("# Org\n\nALWAYS keep the audit record.\n");
        written(&mut connection, tables.parts());
        let mut expected: Vec<_> = tables
            .parts()
            .iter()
            .map(|row| {
                (
                    row.id().to_string(),
                    row.steering_plan_id().to_string(),
                    row.phase().to_string(),
                    i64::try_from(row.part_index()).unwrap(),
                    row.rules_content().to_string(),
                )
            })
            .collect();
        expected.sort_by(|left, right| (&left.2, left.3).cmp(&(&right.2, right.3)));
        assert_eq!(read_back(&connection), expected);
        assert_eq!(expected.len(), 5, "1 部 × 5 フェーズ");
    }

    #[test]
    fn a_second_replacement_leaves_no_part_of_the_first() {
        let big = "x".repeat(12 * 1024);
        let mut connection = Connection::open_in_memory().unwrap();
        written(
            &mut connection,
            tables(&format!("# A\n{big}\n# B\n{big}\n")).parts(),
        );
        assert_eq!(read_back(&connection).len(), 10, "2 部 × 5 フェーズ");
        let empty = SteeringTables::pack(&MemoryRules::default()).unwrap();
        written(&mut connection, empty.parts());
        assert!(read_back(&connection).is_empty(), "古い部が残らない");
    }

    #[test]
    fn the_table_has_a_single_id_key_and_its_natural_and_lookup_indexes() {
        let mut connection = Connection::open_in_memory().unwrap();
        written(&mut connection, &[]);
        written(&mut connection, &[]);
        let key: Vec<String> = connection
            .prepare("SELECT name FROM pragma_table_info('read_steering_part') WHERE pk > 0")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(key, ["id"]);
        let index_columns = |index: &str| -> Vec<String> {
            connection
                .prepare(&format!(
                    "SELECT name FROM pragma_index_info('{index}') ORDER BY seqno"
                ))
                .unwrap()
                .query_map([], |row| row.get(0))
                .unwrap()
                .map(Result::unwrap)
                .collect()
        };
        assert_eq!(
            index_columns("read_steering_part_key"),
            ["phase", "part_index"]
        );
        assert_eq!(
            index_columns("read_steering_part_plan"),
            ["steering_plan_id", "part_index"]
        );
        let unique: bool = connection
            .query_row(
                "SELECT \"unique\" FROM pragma_index_list('read_steering_part') WHERE name = 'read_steering_part_key'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(unique, "自然キーの重複を止めるのはこの索引だけである");
    }
}
