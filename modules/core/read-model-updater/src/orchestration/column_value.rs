//! 表の DAO 実装が共有する、値と列のあいだの写像 (公開型ゼロの内部モジュール)。
//!
//! 文 (SQL) はここに置かない — 引く文はそれぞれの表の DAO が持ち、ここへ渡す。ここが持つのは
//! 「数を `INTEGER` へ写す」「読んだ行を生の値のまま運ぶ」といった、表によらない写像だけである。

use rusqlite::Connection;
use rusqlite::types::Value;

use super::journal_reader_impl::corrupt_error;
use super::store_failure::SqliteResultExt;
use super::{CorruptCause, GlobalSeqNr, JournalReadError, TableContent};

/// 集約に属さない値 (走査位置など) の識別子欄に置く印。
const NO_AGGREGATE: &str = "-";

/// 数を SQLite の `INTEGER` (i64) へ写す。収まらない値を静かに丸めるのは行に嘘を書くことなので、
/// 行の主キーを添えて `Corrupt` で止める。
pub(super) fn integer(value: usize, row_id: &str) -> Result<i64, JournalReadError> {
    i64::try_from(value).map_err(|_| corrupt_error(row_id, None, CorruptCause::InvariantViolation))
}

/// 省略可能な数を `INTEGER | NULL` へ写す。
pub(super) fn optional_integer(
    value: Option<usize>,
    row_id: &str,
) -> Result<Option<i64>, JournalReadError> {
    value.map(|value| integer(value, row_id)).transpose()
}

/// 走査位置 (`as_of` など) を `INTEGER` へ写す。収まらない位置は `Corrupt` で止める。
pub(super) fn scan_position(position: GlobalSeqNr) -> Result<i64, JournalReadError> {
    i64::try_from(position.to_u64())
        .map_err(|_| corrupt_error(NO_AGGREGATE, None, CorruptCause::InvariantViolation))
}

/// `SELECT COUNT(*) FROM sqlite_master …` の答えを「在るか」へ写す (書込ロックを取らない)。
pub(super) fn table_exists(connection: &Connection, sql: &str) -> Result<bool, JournalReadError> {
    let found: i64 = connection
        .query_row(sql, [], |row| row.get(0))
        .at_connection(connection)?;
    Ok(found > 0)
}

/// `SELECT COUNT(*) FROM sqlite_master …` の答え (表と索引の本数)。書込ロックを取らない。
pub(super) fn count_schema_objects(
    connection: &Connection,
    sql: &str,
) -> Result<i64, JournalReadError> {
    connection
        .query_row(sql, [], |row| row.get(0))
        .at_connection(connection)
}

/// 全行を生の値のまま読む (列の並びは文が返す並び)。
pub(super) fn read_content(
    connection: &Connection,
    sql: &str,
) -> Result<TableContent, JournalReadError> {
    let mut statement = connection.prepare(sql).at_connection(connection)?;
    let columns = statement.column_count();
    let rows = statement
        .query_map([], |row| {
            (0..columns)
                .map(|column| row.get::<_, Value>(column))
                .collect::<Result<Vec<Value>, _>>()
        })
        .at_connection(connection)?;
    let mut collected = Vec::new();
    for row in rows {
        collected.push(row.at_connection(connection)?);
    }
    Ok(TableContent::new(collected))
}

/// `SELECT MAX(…)` の答えを走査位置へ写す (行が無ければ `None`、負の値は `Corrupt`)。
pub(super) fn max_position(
    connection: &Connection,
    sql: &str,
) -> Result<Option<GlobalSeqNr>, JournalReadError> {
    let found: Option<i64> = connection
        .query_row(sql, [], |row| row.get(0))
        .at_connection(connection)?;
    found
        .map(|value| {
            u64::try_from(value)
                .map(GlobalSeqNr::new)
                .map_err(|_| corrupt_error(NO_AGGREGATE, None, CorruptCause::InvariantViolation))
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_count_beyond_the_column_is_corrupt_under_the_row_it_belongs_to() {
        assert_eq!(integer(3, "row").unwrap(), 3);
        assert_eq!(
            integer(usize::MAX, "row").unwrap_err(),
            corrupt_error("row", None, CorruptCause::InvariantViolation)
        );
        assert_eq!(optional_integer(None, "row").unwrap(), None);
        assert_eq!(optional_integer(Some(2), "row").unwrap(), Some(2));
    }

    #[test]
    fn a_scan_position_beyond_the_column_is_corrupt_rather_than_rounded() {
        assert_eq!(scan_position(GlobalSeqNr::new(7)).unwrap(), 7);
        assert_eq!(
            scan_position(GlobalSeqNr::new(u64::MAX)).unwrap_err(),
            corrupt_error(NO_AGGREGATE, None, CorruptCause::InvariantViolation)
        );
    }

    #[test]
    fn the_maximum_of_an_empty_column_is_none_and_a_negative_one_is_corrupt() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("CREATE TABLE t (v INTEGER)")
            .unwrap();
        assert_eq!(
            max_position(&connection, "SELECT MAX(v) FROM t").unwrap(),
            None
        );
        connection
            .execute_batch("INSERT INTO t VALUES (4)")
            .unwrap();
        assert_eq!(
            max_position(&connection, "SELECT MAX(v) FROM t").unwrap(),
            Some(GlobalSeqNr::new(4))
        );
        connection
            .execute_batch("DELETE FROM t; INSERT INTO t VALUES (-1)")
            .unwrap();
        assert_eq!(
            max_position(&connection, "SELECT MAX(v) FROM t").unwrap_err(),
            corrupt_error(NO_AGGREGATE, None, CorruptCause::InvariantViolation)
        );
    }

    #[test]
    fn the_content_carries_every_value_with_its_storage_type() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE t (id TEXT PRIMARY KEY, n INTEGER, s TEXT);
                 INSERT INTO t VALUES ('b', NULL, 'x'), ('a', 1, NULL);",
            )
            .unwrap();
        let content = read_content(&connection, "SELECT * FROM t ORDER BY id").unwrap();
        assert_eq!(
            content.rows(),
            [
                vec![Value::Text("a".into()), Value::Integer(1), Value::Null],
                vec![
                    Value::Text("b".into()),
                    Value::Null,
                    Value::Text("x".into())
                ],
            ]
        );
    }
}
