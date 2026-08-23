//! ストアのスキーマ (C6) と `PRAGMA user_version` による版の検査・初期化 (BR2.1 / BR2.2)。
//!
//! DDL は契約 C6 の**逐語**である。列を足さない・索引を足さない (`UNIQUE(aggregate_id,
//! seq_nr)` の暗黙索引だけが例外) のは、格納形式が U3 と U4 の共有契約だからで、
//! ここを勝手に広げると投影側 (U4) の前提が黙って壊れる。DDL の変更は契約の改訂として扱う。
//!
//! 版は `PRAGMA user_version` に刻む。ファイルは 1 つで、開いた側が「読める形か」を
//! 開始時に判定できる必要がある — 判定できないまま読み書きすると、将来版のファイルを
//! 旧実装が壊す経路ができる。

use rusqlite::Connection;

use core_use_case::orchestration::EventStoreError;

use super::sqlite_event_store::map_sqlite_error;

/// 本実装が読み書きできるストアの版 (`PRAGMA user_version`)。
pub(crate) const SUPPORTED_USER_VERSION: u32 = 1;

/// まだスキーマが無いストアの `user_version` (SQLite の既定値)。
const UNINITIALISED_USER_VERSION: u32 = 0;

/// C6 の DDL (逐語)。3 表と `UNIQUE (aggregate_id, seq_nr)` だけを作る。
pub(crate) const DDL: &str = "\
CREATE TABLE journal (
  global_seq_nr   INTEGER PRIMARY KEY AUTOINCREMENT,
  aggregate_id    TEXT    NOT NULL,
  seq_nr          INTEGER NOT NULL,
  schema_version  INTEGER NOT NULL DEFAULT 1,
  event_type      TEXT    NOT NULL,
  payload         TEXT    NOT NULL,
  occurred_at     TEXT    NOT NULL,
  UNIQUE (aggregate_id, seq_nr)
);
CREATE TABLE snapshot (
  aggregate_id    TEXT    PRIMARY KEY,
  version         INTEGER NOT NULL,
  seq_nr          INTEGER NOT NULL,
  schema_version  INTEGER NOT NULL DEFAULT 1,
  payload         TEXT    NOT NULL,
  updated_at      TEXT    NOT NULL
);
CREATE TABLE checkpoint (
  projection      TEXT    PRIMARY KEY,
  last_global_seq INTEGER NOT NULL,
  updated_at      TEXT    NOT NULL
);
";

/// 開いた接続のスキーマを、本実装が読み書きできる状態にする (BR2.1 の前段検査)。
///
/// `user_version` が 0 なら DDL を実行して 1 に刻み、1 ならそのまま使う。それ以外は
/// **触らずに** `Schema` を返す — 知らない版のファイルは読むことも直すこともしない。
///
/// # Errors
///
/// 対応範囲外の版 (`Schema`)、`PRAGMA` / DDL の実行失敗 (`Io` ほか — `map_sqlite_error`
/// の写像) を返す。
pub(crate) fn ensure_schema(
    conn: &Connection,
    path: &std::path::Path,
) -> Result<(), EventStoreError> {
    let found = read_user_version(conn, path)?;
    if found == SUPPORTED_USER_VERSION {
        return Ok(());
    }
    if found != UNINITIALISED_USER_VERSION {
        return Err(EventStoreError::Schema {
            found,
            supported: SUPPORTED_USER_VERSION,
        });
    }
    // 作成と版の刻印は 1 つの Tx に閉じる (途中で落ちたら「版 0 のまま表も無い」に戻る)。
    conn.execute_batch(&format!(
        "BEGIN IMMEDIATE;\n{DDL}PRAGMA user_version = {SUPPORTED_USER_VERSION};\nCOMMIT;"
    ))
    .map_err(|error| map_sqlite_error(&error, path))
}

/// `PRAGMA user_version` を読む (負値・範囲外は「知らない版」として扱う)。
fn read_user_version(conn: &Connection, path: &std::path::Path) -> Result<u32, EventStoreError> {
    let raw: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|error| map_sqlite_error(&error, path))?;
    Ok(u32::try_from(raw).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_memory() -> Connection {
        Connection::open_in_memory().expect("in-memory 接続")
    }

    #[test]
    fn the_ddl_creates_exactly_the_three_contract_tables() {
        let conn = in_memory();
        ensure_schema(&conn, std::path::Path::new(":memory:")).expect("初期化");

        let mut statement = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .expect("sqlite_master");
        let tables: Vec<String> = statement
            .query_map([], |row| row.get::<_, String>(0))
            .expect("表の一覧")
            .filter_map(Result::ok)
            .filter(|name| !name.starts_with("sqlite_"))
            .collect();
        assert_eq!(tables, ["checkpoint", "journal", "snapshot"]);
    }

    #[test]
    fn initialising_stamps_the_supported_version() {
        let conn = in_memory();
        ensure_schema(&conn, std::path::Path::new(":memory:")).expect("初期化");
        let version: i64 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("user_version");
        assert_eq!(version, i64::from(SUPPORTED_USER_VERSION));
    }

    #[test]
    fn an_already_initialised_store_is_left_alone() {
        let conn = in_memory();
        ensure_schema(&conn, std::path::Path::new(":memory:")).expect("初期化");
        conn.execute(
            "INSERT INTO checkpoint(projection, last_global_seq, updated_at)
             VALUES ('state-file', 3, '2026-08-23T00:00:00Z')",
            [],
        )
        .expect("行を置く");

        ensure_schema(&conn, std::path::Path::new(":memory:")).expect("2 度目も通る");

        let last: i64 = conn
            .query_row("SELECT last_global_seq FROM checkpoint", [], |row| {
                row.get(0)
            })
            .expect("行は残る");
        assert_eq!(last, 3);
    }

    #[test]
    fn an_unknown_version_is_refused_without_touching_the_store() {
        let conn = in_memory();
        conn.pragma_update(None, "user_version", 7_i64)
            .expect("将来版を騙る");

        let err = ensure_schema(&conn, std::path::Path::new(":memory:"))
            .expect_err("知らない版は読まない");
        assert_eq!(
            err,
            EventStoreError::Schema {
                found: 7,
                supported: 1
            }
        );

        let tables: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table'",
                [],
                |row| row.get(0),
            )
            .expect("件数");
        assert_eq!(tables, 0, "表を作らない");
    }

    #[test]
    fn a_negative_user_version_is_treated_as_an_unknown_version() {
        let conn = in_memory();
        conn.pragma_update(None, "user_version", -1_i64)
            .expect("負値を書く");
        let err =
            ensure_schema(&conn, std::path::Path::new(":memory:")).expect_err("負値も知らない版");
        assert!(matches!(err, EventStoreError::Schema { .. }), "{err:?}");
    }
}
