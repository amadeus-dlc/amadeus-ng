//! `SteeringPlanDao` の SQLite 実装 — `read_steering_plan` 表 1 つだけを読み書きする。

use rusqlite::{Connection, OptionalExtension as _, Transaction, params};

use super::journal_reader_impl::corrupt_error;
use super::store_failure::SqliteResultExt;
use super::{CorruptCause, JournalReadError, SteeringPlanDao, SteeringPlanRow};

/// 表と索引の DDL (冪等)。
///
/// 主キーは自然キー `phase` から導いた代理キー `id` で、`phase` の重複は UNIQUE 索引が
/// 止める。クエリ側が `bundle_digest` で引くのでセカンダリ索引を張る。
///
/// `as_of` 列を持たない — この面は参照入力由来で、ジャーナルの走査位置と無関係である。
/// どの参照入力から作られたかは `source_digest` が名乗る。
const CREATE_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS read_steering_plan (
  id              TEXT PRIMARY KEY,
  phase           TEXT    NOT NULL,
  bundle_digest   TEXT    NOT NULL,
  part_count      INTEGER NOT NULL,
  delivered_paths TEXT    NOT NULL,
  source_digest   TEXT    NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS read_steering_plan_key
  ON read_steering_plan(phase);
CREATE INDEX IF NOT EXISTS read_steering_plan_bundle_digest
  ON read_steering_plan(bundle_digest);";

/// 保存済みの出所 (全行に同じ値が書かれているので 1 行で足りる)。
const SELECT_SOURCE_DIGEST: &str =
    "SELECT source_digest FROM read_steering_plan ORDER BY phase LIMIT 1";

/// 全行を消す (全差し替えの前半)。
const DELETE_ALL: &str = "DELETE FROM read_steering_plan";

/// 行を足す。
const INSERT: &str = "INSERT INTO read_steering_plan
     (id, phase, bundle_digest, part_count, delivered_paths, source_digest)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6)";

/// 表が在るか (`sqlite_master` を引くだけ — 書込ロックを取らない)。
const TABLE_EXISTS: &str =
    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'read_steering_plan'";

/// `read_steering_plan` 表の DAO の実装。状態を持たない。
#[derive(Debug, Clone, Copy, Default)]
pub struct SteeringPlanDaoImpl;

impl SteeringPlanDao for SteeringPlanDaoImpl {
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

    fn find_source_digest(
        &self,
        connection: &Connection,
    ) -> Result<Option<String>, JournalReadError> {
        connection
            .query_row(SELECT_SOURCE_DIGEST, [], |row| row.get(0))
            .optional()
            .at_connection(connection)
    }

    fn replace(
        &self,
        transaction: &mut Transaction<'_>,
        rows: &[SteeringPlanRow],
        source_digest: &str,
    ) -> Result<(), JournalReadError> {
        transaction
            .execute(DELETE_ALL, [])
            .at_connection(transaction)?;
        for row in rows {
            let part_count = i64::try_from(row.part_count())
                .map_err(|_| corrupt_error(row.id(), None, CorruptCause::InvariantViolation))?;
            transaction
                .execute(
                    INSERT,
                    params![
                        row.id(),
                        row.phase(),
                        row.bundle_digest(),
                        part_count,
                        row.delivered_paths(),
                        source_digest,
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

    use super::*;
    use crate::read_tables::{MemoryRules, RuleContent, SteeringTables};

    fn tables(text: &str) -> SteeringTables {
        SteeringTables::pack(&MemoryRules::new(
            vec![RuleContent::new("org.md".to_string(), text.to_string())],
            BTreeMap::new(),
        ))
        .unwrap()
    }

    fn written(connection: &mut Connection, tables: &SteeringTables) {
        let mut transaction = connection.transaction().unwrap();
        SteeringPlanDaoImpl.create_table(&mut transaction).unwrap();
        SteeringPlanDaoImpl
            .replace(&mut transaction, tables.plans(), tables.source_digest())
            .unwrap();
        transaction.commit().unwrap();
    }

    fn created(connection: &mut Connection) {
        let mut transaction = connection.transaction().unwrap();
        SteeringPlanDaoImpl.create_table(&mut transaction).unwrap();
        transaction.commit().unwrap();
    }

    fn read_back(connection: &Connection) -> Vec<(String, String, String, i64, String, String)> {
        let mut statement = connection
            .prepare(
                "SELECT id, phase, bundle_digest, part_count, delivered_paths, source_digest
                 FROM read_steering_plan ORDER BY phase",
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
    fn the_replaced_rows_read_back_column_for_column_with_the_source_they_came_from() {
        let mut connection = Connection::open_in_memory().unwrap();
        let tables = tables("# Org\n\nALWAYS keep the audit record.\n");
        written(&mut connection, &tables);
        let mut expected: Vec<_> = tables
            .plans()
            .iter()
            .map(|row| {
                (
                    row.id().to_string(),
                    row.phase().to_string(),
                    row.bundle_digest().to_string(),
                    i64::try_from(row.part_count()).unwrap(),
                    row.delivered_paths().to_string(),
                    tables.source_digest().to_string(),
                )
            })
            .collect();
        expected.sort_by(|left, right| left.1.cmp(&right.1));
        assert_eq!(read_back(&connection), expected);
        assert_eq!(
            SteeringPlanDaoImpl.find_source_digest(&connection).unwrap(),
            Some(tables.source_digest().to_string())
        );
    }

    #[test]
    fn an_empty_table_names_no_source() {
        let mut connection = Connection::open_in_memory().unwrap();
        created(&mut connection);
        assert_eq!(
            SteeringPlanDaoImpl.find_source_digest(&connection).unwrap(),
            None
        );
    }

    #[test]
    fn a_second_replacement_leaves_no_row_of_the_first() {
        let mut connection = Connection::open_in_memory().unwrap();
        let first = tables("# Org\n\n最初の規則\n");
        let second = tables("# Org\n\n直した規則\n");
        written(&mut connection, &first);
        written(&mut connection, &second);
        let rows = read_back(&connection);
        assert_eq!(rows.len(), second.plans().len());
        assert!(
            rows.iter()
                .all(|row| row.5 == second.source_digest() && row.5 != first.source_digest()),
            "古い出所の行が残らない"
        );
    }

    #[test]
    fn creating_the_table_twice_is_harmless() {
        let mut connection = Connection::open_in_memory().unwrap();
        created(&mut connection);
        written(&mut connection, &tables("# Org\n\n規則\n"));
        created(&mut connection);
        assert_eq!(read_back(&connection).len(), 5, "5 フェーズぶんの行が残る");
    }

    #[test]
    fn the_table_has_a_single_id_key_a_unique_phase_and_no_scan_position() {
        let mut connection = Connection::open_in_memory().unwrap();
        created(&mut connection);
        let key: Vec<String> = connection
            .prepare("SELECT name FROM pragma_table_info('read_steering_plan') WHERE pk > 0")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(key, ["id"]);
        let indexes: Vec<(String, bool)> = connection
            .prepare("SELECT name, \"unique\" FROM pragma_index_list('read_steering_plan') WHERE origin = 'c' ORDER BY name")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            indexes,
            [
                ("read_steering_plan_bundle_digest".to_string(), false),
                ("read_steering_plan_key".to_string(), true),
            ]
        );
        let as_of: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('read_steering_plan') WHERE name = 'as_of'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(as_of, 0, "参照入力由来の面は走査位置を持たない");
    }
}
