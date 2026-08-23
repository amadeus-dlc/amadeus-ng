//! `EventStoreImpl` の実装固有の契約 (BR2.1 / BR2.2 / BR2.3 / BR2.4 / BR1.4)。
//!
//! ポートの面から見える約束は `workflow_execution_repository_contract.rs` が in-memory と
//! 共有して検査する。本ファイルが持つのは**SQLite 実装にしか無い観測**である:
//! スキーマ (C6 逐語) の突合、`user_version` の検査と初期化、`BEGIN IMMEDIATE` が
//! 他接続を締め出すこと、`busy_timeout` 超過の写像、そして行を直接壊したときの振る舞い。

// テストコードでは unwrap / expect を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::io::ErrorKind;
use std::path::Path;
use std::time::Duration;

use core_domain::orchestration::{IntentId, WorkflowExecution, WorkflowExecutionEvent};
use core_domain::workspace::SpaceName;
use core_interface_adapter::FakeClock;
use core_interface_adapter::orchestration::{EventStoreImpl, StorePath};
use core_use_case::orchestration::{
    EventStore, EventStoreError, GlobalSeqNr, JournalReader, ProjectionName,
};
use rusqlite::Connection;
use tempfile::TempDir;

use support::{AT, advanced, genesis, intent_id};

/// 固定時刻 (2026-08-23T00:00:00Z の epoch ms) — `updated_at` の期待値を決める。
const NOW_MS: u64 = 1_787_443_200_000;
/// `NOW_MS` を ISO 8601 UTC で描いた形。
const NOW_ISO: &str = "2026-08-23T00:00:00Z";

/// 一時ディレクトリ配下に `spaces/<space>/intents/` を作り、そこへストアを開く試験装置。
struct Fixture {
    _dir: TempDir,
    path: StorePath,
}

impl Fixture {
    fn new() -> Fixture {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        let path = StorePath::for_space(&dir.path().join("aidlc"), &SpaceName::default_space());
        // `intents/` は upstream の既存ディレクトリ — ストアは作らない (BR2.1)。
        std::fs::create_dir_all(path.as_path().parent().expect("親 dir を持つ"))
            .expect("intents/ を先に作る");
        Fixture { _dir: dir, path }
    }

    /// 親 dir を作らない試験装置 (`Io(NotFound)` の観測用)。
    fn without_parent() -> Fixture {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        let path = StorePath::for_space(&dir.path().join("aidlc"), &SpaceName::default_space());
        Fixture { _dir: dir, path }
    }

    fn store(&self) -> EventStoreImpl<FakeClock> {
        EventStoreImpl::open(self.path.clone(), FakeClock::new(NOW_MS)).expect("ストアは開ける")
    }

    fn store_with_busy_timeout(&self, busy_timeout: Duration) -> EventStoreImpl<FakeClock> {
        EventStoreImpl::open_with_busy_timeout(
            self.path.clone(),
            FakeClock::new(NOW_MS),
            busy_timeout,
        )
        .expect("ストアは開ける")
    }

    /// 行を直接読み書きするための生の接続 (実装を経由しない観測・破壊の唯一の口)。
    fn raw(&self) -> Connection {
        Connection::open(self.path.as_path()).expect("生の接続")
    }
}

/// `PRAGMA table_info` の 1 行 (列名・宣言型・NOT NULL・主キー位置)。
fn table_info(conn: &Connection, table: &str) -> Vec<(String, String, bool, i64)> {
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .expect("table_info");
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)? != 0,
                row.get::<_, i64>(5)?,
            ))
        })
        .expect("table_info の行");
    rows.map(|row| row.expect("行")).collect()
}

/// `sqlite_master` に記録された CREATE 文。
fn create_sql(conn: &Connection, table: &str) -> String {
    conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table],
        |row| row.get::<_, String>(0),
    )
    .expect("CREATE 文")
}

/// genesis を書いたストアと、書込後に版を載せ替えた集約を返す。
async fn seeded(fixture: &Fixture) -> (EventStoreImpl<FakeClock>, WorkflowExecution) {
    let mut store = fixture.store();
    let (aggregate, event) = genesis();
    store
        .persist_event_and_snapshot(&event, &aggregate)
        .await
        .expect("genesis の書込");
    (store, advanced(aggregate, &event))
}

// ---------------------------------------------------------------------------
// open と初期化 (BR2.1 / BR2.2)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_new_store_creates_the_three_tables_and_stamps_the_schema_version() {
    let fixture = Fixture::new();
    let _store = fixture.store();

    let conn = fixture.raw();
    let user_version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("user_version");
    assert_eq!(user_version, 1, "初期化したストアの版は 1");

    let mut statement = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .expect("sqlite_master");
    let tables: Vec<String> = statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("表の一覧")
        .map(|row| row.expect("行"))
        .filter(|name| !name.starts_with("sqlite_"))
        .collect();
    assert_eq!(tables, ["checkpoint", "journal", "snapshot"]);
}

#[tokio::test]
async fn an_existing_store_opens_again_without_touching_its_rows() {
    let fixture = Fixture::new();
    let (_store, expected) = seeded(&fixture).await;

    let reopened = fixture.store();
    let found = reopened
        .get_latest_snapshot_by_id(&intent_id())
        .await
        .expect("読める")
        .expect("行がある");
    assert_eq!(found.state(), expected.state());
}

#[tokio::test]
async fn a_store_written_by_a_newer_schema_is_refused() {
    let fixture = Fixture::new();
    {
        let conn = fixture.raw();
        conn.pragma_update(None, "user_version", 2_i64)
            .expect("将来版を騙る");
    }
    let err = EventStoreImpl::open(fixture.path, FakeClock::new(NOW_MS))
        .expect_err("対応外の版は開かない");
    assert_eq!(
        err,
        EventStoreError::Schema {
            found: 2,
            supported: 1
        }
    );
}

#[tokio::test]
async fn a_missing_parent_directory_is_reported_as_io_not_found() {
    let fixture = Fixture::without_parent();
    let err = EventStoreImpl::open(fixture.path.clone(), FakeClock::new(NOW_MS))
        .expect_err("親 dir を作らない (BR2.1)");
    assert!(
        matches!(
            err,
            EventStoreError::Io {
                kind: ErrorKind::NotFound,
                ..
            }
        ),
        "実際: {err:?}"
    );
    assert!(!fixture.path.as_path().exists(), "ファイルも作らない");
}

#[tokio::test]
async fn the_journal_table_matches_the_contract() {
    let fixture = Fixture::new();
    let _store = fixture.store();
    let conn = fixture.raw();

    assert_eq!(
        table_info(&conn, "journal"),
        [
            ("global_seq_nr".to_string(), "INTEGER".to_string(), false, 1),
            ("aggregate_id".to_string(), "TEXT".to_string(), true, 0),
            ("seq_nr".to_string(), "INTEGER".to_string(), true, 0),
            ("schema_version".to_string(), "INTEGER".to_string(), true, 0),
            ("event_type".to_string(), "TEXT".to_string(), true, 0),
            ("payload".to_string(), "TEXT".to_string(), true, 0),
            ("occurred_at".to_string(), "TEXT".to_string(), true, 0),
        ]
    );
    let sql = create_sql(&conn, "journal");
    assert!(sql.contains("AUTOINCREMENT"), "global 通番は AUTOINCREMENT");
    assert!(
        sql.contains("UNIQUE (aggregate_id, seq_nr)"),
        "集約内の重複を DB が拒む"
    );
}

#[tokio::test]
async fn the_snapshot_table_matches_the_contract() {
    let fixture = Fixture::new();
    let _store = fixture.store();
    let conn = fixture.raw();

    assert_eq!(
        table_info(&conn, "snapshot"),
        [
            ("aggregate_id".to_string(), "TEXT".to_string(), false, 1),
            ("version".to_string(), "INTEGER".to_string(), true, 0),
            ("seq_nr".to_string(), "INTEGER".to_string(), true, 0),
            ("schema_version".to_string(), "INTEGER".to_string(), true, 0),
            ("payload".to_string(), "TEXT".to_string(), true, 0),
            ("updated_at".to_string(), "TEXT".to_string(), true, 0),
        ]
    );
}

#[tokio::test]
async fn the_checkpoint_table_matches_the_contract() {
    let fixture = Fixture::new();
    let _store = fixture.store();
    let conn = fixture.raw();

    assert_eq!(
        table_info(&conn, "checkpoint"),
        [
            ("projection".to_string(), "TEXT".to_string(), false, 1),
            (
                "last_global_seq".to_string(),
                "INTEGER".to_string(),
                true,
                0
            ),
            ("updated_at".to_string(), "TEXT".to_string(), true, 0),
        ]
    );
}

#[tokio::test]
async fn the_connection_waits_five_seconds_for_a_busy_store() {
    let fixture = Fixture::new();
    let _store = fixture.store();
    // `busy_timeout` は接続ごとの設定なので、実装が開いた接続からしか観測できない。
    // 値そのものの検査は `event_store_impl.rs` のインラインテストが持つ。
    // ここでは「別の値を指定して開ける口がある」ことだけを固定する (短い timeout の観測用)。
    let _short = fixture.store_with_busy_timeout(Duration::from_millis(20));
}

// ---------------------------------------------------------------------------
// 書込 (BR2.3)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_genesis_write_inserts_one_journal_row_and_one_snapshot_row() {
    let fixture = Fixture::new();
    let (_store, _) = seeded(&fixture).await;
    let conn = fixture.raw();

    let (aggregate_id, seq_nr, schema_version, event_type, occurred_at): (
        String,
        i64,
        i64,
        String,
        String,
    ) = conn
        .query_row(
            "SELECT aggregate_id, seq_nr, schema_version, event_type, occurred_at FROM journal",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("ジャーナル行");
    assert_eq!(aggregate_id, intent_id().as_str());
    assert_eq!(seq_nr, 1);
    assert_eq!(schema_version, 1);
    assert_eq!(event_type, "Started");
    assert_eq!(occurred_at, AT, "呼出側が渡した時刻を素通しする (BR2.6)");

    let (version, snapshot_seq, updated_at): (i64, i64, String) = conn
        .query_row(
            "SELECT version, seq_nr, updated_at FROM snapshot",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("スナップショット行");
    assert_eq!(version, 1);
    assert_eq!(snapshot_seq, 1);
    assert_eq!(updated_at, NOW_ISO, "updated_at は注入した時計から (BR2.6)");
}

#[tokio::test]
async fn the_snapshot_payload_carries_the_new_version() {
    let fixture = Fixture::new();
    let (_store, _) = seeded(&fixture).await;
    let payload: String = fixture
        .raw()
        .query_row("SELECT payload FROM snapshot", [], |row| row.get(0))
        .expect("スナップショット payload");
    assert!(
        payload.contains("\"version\":1"),
        "payload の version も列と同じ新 version に揃える。実際: {payload}"
    );
}

#[tokio::test]
async fn a_second_write_updates_the_snapshot_in_place_and_appends_to_the_journal() {
    let fixture = Fixture::new();
    let (mut store, mut aggregate) = seeded(&fixture).await;

    let event = aggregate.complete_stage(AT).expect("索引 0 は非ゲート");
    store
        .persist_event_and_snapshot(&event, &aggregate)
        .await
        .expect("2 件目");

    let conn = fixture.raw();
    let journal_rows: i64 = conn
        .query_row("SELECT count(*) FROM journal", [], |row| row.get(0))
        .expect("件数");
    let snapshot_rows: i64 = conn
        .query_row("SELECT count(*) FROM snapshot", [], |row| row.get(0))
        .expect("件数");
    assert_eq!(journal_rows, 2, "ジャーナルは追記");
    assert_eq!(snapshot_rows, 1, "スナップショットは集約 1 行のまま");

    let (version, seq_nr): (i64, i64) = conn
        .query_row("SELECT version, seq_nr FROM snapshot", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .expect("スナップショット行");
    assert_eq!((version, seq_nr), (2, 2));
}

#[tokio::test]
async fn a_duplicate_sequence_number_conflicts_and_rolls_the_transaction_back() {
    let fixture = Fixture::new();
    let (mut store, _) = seeded(&fixture).await;
    let (aggregate, event) = genesis();

    let err = store
        .persist_event_and_snapshot(&event, &aggregate)
        .await
        .expect_err("同じ seq_nr は 2 度書けない");
    assert_eq!(
        err,
        EventStoreError::Conflict {
            expected: 0,
            actual: 1
        }
    );

    let journal_rows: i64 = fixture
        .raw()
        .query_row("SELECT count(*) FROM journal", [], |row| row.get(0))
        .expect("件数");
    assert_eq!(journal_rows, 1, "rollback してジャーナル行は残らない");
}

#[tokio::test]
async fn a_stale_version_conflicts_and_rolls_the_transaction_back() {
    let fixture = Fixture::new();
    let (mut store, aggregate) = seeded(&fixture).await;

    // 同じ版から 2 つの書込を作る (2 再水和の競合)。
    let mut first = aggregate.clone();
    let mut second = aggregate;
    let event = first.complete_stage(AT).expect("索引 0 は非ゲート");
    store
        .persist_event_and_snapshot(&event, &first)
        .await
        .expect("先に書いた方は通る");

    let event = second.complete_stage(AT).expect("同じコマンド");
    let err = store
        .persist_event_and_snapshot(&event, &second)
        .await
        .expect_err("後から書いた方は衝突");
    assert_eq!(
        err,
        EventStoreError::Conflict {
            expected: 1,
            actual: 2
        }
    );

    let journal_rows: i64 = fixture
        .raw()
        .query_row("SELECT count(*) FROM journal", [], |row| row.get(0))
        .expect("件数");
    assert_eq!(journal_rows, 2, "rollback してジャーナル行は増えない");
}

#[tokio::test]
async fn the_append_only_write_checks_the_optimistic_version_without_touching_the_snapshot() {
    let fixture = Fixture::new();
    let (mut store, mut aggregate) = seeded(&fixture).await;
    let next = aggregate.complete_stage(AT).expect("索引 0 は非ゲート");

    let err = store
        .persist_event(&next, 0)
        .await
        .expect_err("版 0 を前提にはできない");
    assert_eq!(
        err,
        EventStoreError::Conflict {
            expected: 0,
            actual: 1
        }
    );

    store.persist_event(&next, 1).await.expect("版 1 なら通る");

    let conn = fixture.raw();
    let journal_rows: i64 = conn
        .query_row("SELECT count(*) FROM journal", [], |row| row.get(0))
        .expect("件数");
    let version: i64 = conn
        .query_row("SELECT version FROM snapshot", [], |row| row.get(0))
        .expect("スナップショット行");
    assert_eq!(journal_rows, 2, "ジャーナルには追記されている");
    assert_eq!(version, 1, "スナップショットは動かない");
}

// ---------------------------------------------------------------------------
// 読取 (BR1.2 / BR1.4)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_event_read_returns_the_tail_of_one_aggregate_in_order() {
    let fixture = Fixture::new();
    let (mut store, mut aggregate) = seeded(&fixture).await;
    let event = aggregate.complete_stage(AT).expect("索引 0 は非ゲート");
    store
        .persist_event_and_snapshot(&event, &aggregate)
        .await
        .expect("2 件目");
    let aggregate = advanced(aggregate, &event);
    let mut aggregate = aggregate;
    let event = aggregate
        .open_gate(vec!["intent.md".to_string()], AT)
        .expect("索引 1 はゲート付き");
    store
        .persist_event_and_snapshot(&event, &aggregate)
        .await
        .expect("3 件目");

    let events = store
        .get_events_by_id_since_seq_nr(&intent_id(), 1)
        .await
        .expect("差分読取");
    assert_eq!(
        events
            .iter()
            .map(WorkflowExecutionEvent::seq_nr)
            .collect::<Vec<_>>(),
        [2, 3]
    );
}

#[tokio::test]
async fn the_journal_read_spans_every_aggregate_in_global_order() {
    let fixture = Fixture::new();
    let (store, _) = seeded(&fixture).await;

    // 別集約の行を直接足し、global 通番の横断性を観測する。
    fixture
        .raw()
        .execute(
            "INSERT INTO journal(aggregate_id, seq_nr, schema_version, event_type, payload, occurred_at)
             VALUES (?1, 1, 1, 'Unparked', '{\"type\":\"Unparked\"}', ?2)",
            rusqlite::params!["018f3b2c-4d5e-7f60-8abc-def012345678", AT],
        )
        .expect("別集約の行");

    let rows = store.events_after(GlobalSeqNr::ZERO).await.expect("全件");
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows.iter()
            .map(|(global, _)| global.value())
            .collect::<Vec<_>>(),
        [1, 2],
        "global 通番の昇順"
    );
    assert_eq!(
        rows.iter()
            .map(|(_, event)| event.intent_id().as_str().to_string())
            .collect::<Vec<_>>(),
        [
            "01a02785-1bd8-76eb-aeea-5aa303ebd5b6",
            "018f3b2c-4d5e-7f60-8abc-def012345678"
        ]
    );
}

#[tokio::test]
async fn a_journal_row_naming_an_unparsable_aggregate_is_corrupt() {
    let fixture = Fixture::new();
    let (store, _) = seeded(&fixture).await;
    fixture
        .raw()
        .execute("UPDATE journal SET aggregate_id = 'not-a-uuid'", [])
        .expect("識別子を壊す");

    let err = store
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect_err("壊れた識別子は読めない");
    assert!(
        matches!(err, EventStoreError::Corrupt { .. }),
        "実際: {err:?}"
    );
}

#[tokio::test]
async fn the_checkpoint_starts_at_zero_and_advances_monotonically() {
    let fixture = Fixture::new();
    let (mut store, _) = seeded(&fixture).await;
    let projection = ProjectionName::parse("state-file").expect("kebab");

    assert_eq!(
        store.checkpoint(&projection).await.expect("未登録"),
        GlobalSeqNr::ZERO
    );
    store
        .advance_checkpoint(&projection, GlobalSeqNr::new(1))
        .await
        .expect("前進");
    assert_eq!(
        store.checkpoint(&projection).await.expect("読取"),
        GlobalSeqNr::new(1)
    );

    let updated_at: String = fixture
        .raw()
        .query_row("SELECT updated_at FROM checkpoint", [], |row| row.get(0))
        .expect("チェックポイント行");
    assert_eq!(updated_at, NOW_ISO, "updated_at は注入した時計から");

    let err = store
        .advance_checkpoint(&projection, GlobalSeqNr::ZERO)
        .await
        .expect_err("後退は拒否");
    assert_eq!(
        err,
        EventStoreError::CheckpointRegression {
            projection,
            current: GlobalSeqNr::new(1),
            requested: GlobalSeqNr::ZERO,
        }
    );
}

// ---------------------------------------------------------------------------
// within_write_transaction (BR2.4) と Busy の写像 (NFR3.5)
// ---------------------------------------------------------------------------

/// テストの閉包が返す I/O 失敗 (材料は使わない)。
const fn probe_failure() -> EventStoreError {
    EventStoreError::Io {
        kind: ErrorKind::Other,
        path: None,
    }
}

#[tokio::test]
async fn a_write_transaction_commits_when_the_closure_succeeds() {
    let fixture = Fixture::new();
    let mut store = fixture.store();

    let value = store
        .within_write_transaction(|tx| {
            tx.execute(
                "INSERT INTO checkpoint(projection, last_global_seq, updated_at)
                 VALUES ('probe', 7, '2026-08-23T00:00:00Z')",
                [],
            )
            .map_err(|_| probe_failure())?;
            Ok(42_u32)
        })
        .expect("閉包が Ok なら COMMIT");
    assert_eq!(value, 42);

    let last: i64 = fixture
        .raw()
        .query_row(
            "SELECT last_global_seq FROM checkpoint WHERE projection = 'probe'",
            [],
            |row| row.get(0),
        )
        .expect("行が残る");
    assert_eq!(last, 7);
}

#[tokio::test]
async fn a_write_transaction_rolls_back_when_the_closure_fails() {
    let fixture = Fixture::new();
    let mut store = fixture.store();

    let err = store
        .within_write_transaction(|tx| {
            tx.execute(
                "INSERT INTO checkpoint(projection, last_global_seq, updated_at)
                 VALUES ('probe', 7, '2026-08-23T00:00:00Z')",
                [],
            )
            .map_err(|_| probe_failure())?;
            Err::<u32, EventStoreError>(probe_failure())
        })
        .expect_err("閉包が Err なら rollback");
    assert_eq!(err, probe_failure());

    let rows: i64 = fixture
        .raw()
        .query_row("SELECT count(*) FROM checkpoint", [], |row| row.get(0))
        .expect("件数");
    assert_eq!(rows, 0, "書きかけは残らない");
}

#[tokio::test]
async fn an_open_write_transaction_shuts_other_connections_out() {
    let fixture = Fixture::new();
    let mut store = fixture.store();

    let blocked = store
        .within_write_transaction(|tx| {
            tx.execute(
                "INSERT INTO checkpoint(projection, last_global_seq, updated_at)
                 VALUES ('probe', 1, '2026-08-23T00:00:00Z')",
                [],
            )
            .map_err(|_| probe_failure())?;

            // BEGIN IMMEDIATE で書込ロックを先取りしているので、別接続の書込は待たされる。
            let other = Connection::open(fixture.path.as_path()).map_err(|_| probe_failure())?;
            other
                .busy_timeout(Duration::from_millis(20))
                .map_err(|_| probe_failure())?;
            let attempt = other.execute(
                "INSERT INTO checkpoint(projection, last_global_seq, updated_at)
                 VALUES ('other', 1, '2026-08-23T00:00:00Z')",
                [],
            );
            Ok(attempt.is_err())
        })
        .expect("Tx 自体は成功する");
    assert!(blocked, "Tx 区間の外からは書けない");
}

#[tokio::test]
async fn a_busy_store_reports_would_block_and_succeeds_once_the_holder_commits() {
    let fixture = Fixture::new();
    let mut store = fixture.store_with_busy_timeout(Duration::from_millis(20));
    let (aggregate, event) = genesis();

    // 別接続が書込ロックを握った状態を作る。
    let holder = fixture.raw();
    holder
        .execute_batch("BEGIN IMMEDIATE")
        .expect("書込ロックの先取り");

    let err = store
        .persist_event_and_snapshot(&event, &aggregate)
        .await
        .expect_err("busy_timeout 内に取れなければ諦める");
    assert!(
        matches!(
            err,
            EventStoreError::Io {
                kind: ErrorKind::WouldBlock,
                ..
            }
        ),
        "Busy は WouldBlock に写す (NFR3.5)。実際: {err:?}"
    );

    holder.execute_batch("COMMIT").expect("解放");
    store
        .persist_event_and_snapshot(&event, &aggregate)
        .await
        .expect("解放後は直列化されて通る");
}

// ---------------------------------------------------------------------------
// 破損の検出 (security-design §2)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_snapshot_payload_that_cannot_be_decoded_is_corrupt() {
    let fixture = Fixture::new();
    let (store, _) = seeded(&fixture).await;
    fixture
        .raw()
        .execute("UPDATE snapshot SET payload = '{not json'", [])
        .expect("payload を壊す");

    let err = store
        .get_latest_snapshot_by_id(&intent_id())
        .await
        .expect_err("復号できない");
    assert!(
        matches!(err, EventStoreError::Corrupt { .. }),
        "実際: {err:?}"
    );
}

#[tokio::test]
async fn a_snapshot_row_written_with_another_schema_version_is_corrupt() {
    let fixture = Fixture::new();
    let (store, _) = seeded(&fixture).await;
    fixture
        .raw()
        .execute("UPDATE snapshot SET schema_version = 2", [])
        .expect("版を壊す");

    let err = store
        .get_latest_snapshot_by_id(&intent_id())
        .await
        .expect_err("対応外の版は読めない");
    assert!(
        matches!(err, EventStoreError::Corrupt { .. }),
        "実際: {err:?}"
    );
}

#[tokio::test]
async fn a_journal_row_with_an_unknown_event_type_is_corrupt() {
    let fixture = Fixture::new();
    let (store, _) = seeded(&fixture).await;
    fixture
        .raw()
        .execute("UPDATE journal SET event_type = 'Exploded'", [])
        .expect("変種名を壊す");

    let err = store
        .get_events_by_id_since_seq_nr(&intent_id(), 0)
        .await
        .expect_err("12 語の閉集合の外");
    assert!(
        matches!(err, EventStoreError::Corrupt { .. }),
        "実際: {err:?}"
    );
}

#[tokio::test]
async fn an_unknown_aggregate_has_no_snapshot() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let absent = IntentId::parse("018f3b2c-4d5e-7f60-8abc-def012345678").expect("UUIDv7");
    assert!(
        store
            .get_latest_snapshot_by_id(&absent)
            .await
            .expect("読める")
            .is_none()
    );
}

/// `StorePath` は space 配下の固定位置を導く (BR2.1)。
#[test]
fn the_store_path_is_derived_from_the_space() {
    let path = StorePath::for_space(
        Path::new("/tmp/aidlc"),
        &SpaceName::parse("team-a").expect("kebab"),
    );
    assert_eq!(
        path.as_path(),
        Path::new("/tmp/aidlc/spaces/team-a/intents/.aidlc-store.sqlite")
    );
}
