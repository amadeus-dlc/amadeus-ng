//! `WorkflowExecutionRepositoryImpl` の実装固有の契約 (BR1.2 / BR1.3)。
//!
//! ポートの面から見える約束は `workflow_execution_repository_contract.rs` が in-memory と
//! 共有して検査する。本ファイルが持つのは**行を直接壊してしか作れない状態**の振る舞い
//! (`Corrupt` の 4 原因) と、スナップショットより後ろのイベントを replay する経路である。
//! 破壊は生の SQL で行う — 実装に破壊用のフックを開けない (BR2.8)。

// テストコードでは unwrap / expect を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use core_domain::orchestration::{WorkflowExecution, WorkflowExecutionEvent};
use core_domain::workspace::SpaceName;
use core_interface_adapter::FakeClock;
use core_interface_adapter::orchestration::{
    SqliteEventStore, StorePath, WorkflowExecutionRepositoryImpl,
};
use core_use_case::orchestration::{CorruptCause, RepositoryError, WorkflowExecutionRepository};
use rusqlite::Connection;
use tempfile::TempDir;

use support::{AT, absent_intent_id, advanced, genesis, intent_id};

/// 固定時刻 (2026-08-23T00:00:00Z の epoch ms)。
const NOW_MS: u64 = 1_787_443_200_000;

/// 一時ディレクトリ配下の SQLite ストアと、それを開く Repository。
struct Fixture {
    _dir: TempDir,
    path: StorePath,
}

impl Fixture {
    fn new() -> Fixture {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        let path = StorePath::for_space(&dir.path().join("aidlc"), &SpaceName::default_space());
        std::fs::create_dir_all(path.as_path().parent().expect("親 dir を持つ"))
            .expect("intents/ を先に作る");
        Fixture { _dir: dir, path }
    }

    fn repository(&self) -> WorkflowExecutionRepositoryImpl<FakeClock> {
        WorkflowExecutionRepositoryImpl::new(
            SqliteEventStore::open(self.path.clone(), FakeClock::new(NOW_MS))
                .expect("ストアは開ける"),
        )
    }

    fn raw(&self) -> Connection {
        Connection::open(self.path.as_path()).expect("生の接続")
    }
}

/// genesis + 2 コマンドを書き、最後の集約 (版を載せ替え済み) を返す。
async fn seed(repository: &WorkflowExecutionRepositoryImpl<FakeClock>) -> WorkflowExecution {
    let (mut aggregate, event) = genesis();
    repository.store(&event, &aggregate).await.expect("genesis");
    aggregate = advanced(aggregate, &event);

    let event = aggregate.complete_stage(AT).expect("索引 0 は非ゲート");
    repository.store(&event, &aggregate).await.expect("2 件目");
    aggregate = advanced(aggregate, &event);

    let event = aggregate
        .open_gate(vec!["intent.md".to_string()], AT)
        .expect("索引 1 はゲート付き");
    repository.store(&event, &aggregate).await.expect("3 件目");
    advanced(aggregate, &event)
}

/// スナップショット行を genesis 直後の姿へ巻き戻す (replay 経路を作る唯一の手)。
fn rewind_snapshot_to_genesis(conn: &Connection, payload: &str) {
    conn.execute(
        "UPDATE snapshot SET payload = ?1, version = 1, seq_nr = 1",
        [payload],
    )
    .expect("スナップショットを巻き戻す");
}

/// 現在のスナップショット payload。
fn snapshot_payload(conn: &Connection) -> String {
    conn.query_row("SELECT payload FROM snapshot", [], |row| row.get(0))
        .expect("スナップショット payload")
}

// ---------------------------------------------------------------------------
// 再構成 (BR1.2)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_store_without_any_row_reports_not_found() {
    let fixture = Fixture::new();
    let repository = fixture.repository();
    let err = repository
        .find_by_id(&absent_intent_id())
        .await
        .expect_err("書いていない集約");
    assert_eq!(
        err,
        RepositoryError::NotFound {
            intent_id: absent_intent_id()
        }
    );
}

#[tokio::test]
async fn the_version_after_a_read_without_replay_is_the_last_persisted_sequence() {
    let fixture = Fixture::new();
    let repository = fixture.repository();
    let expected = seed(&repository).await;

    let found = repository.find_by_id(&intent_id()).await.expect("読める");
    assert_eq!(found.version(), 3, "永続化済みの最後の seq_nr");
    assert_eq!(found.seq_nr(), 3);
    assert_eq!(found.state(), expected.state());
}

#[tokio::test]
async fn the_version_after_a_replay_is_the_sequence_of_the_last_applied_event() {
    let fixture = Fixture::new();
    let repository = fixture.repository();
    let expected = seed(&repository).await;

    // genesis 直後の payload を用意して巻き戻すと、seq_nr 2〜3 が replay 経路を通る。
    let genesis_payload = {
        let other = Fixture::new();
        let other_repository = other.repository();
        let (aggregate, event) = genesis();
        other_repository
            .store(&event, &aggregate)
            .await
            .expect("genesis");
        snapshot_payload(&other.raw())
    };
    rewind_snapshot_to_genesis(&fixture.raw(), &genesis_payload);

    let found = repository.find_by_id(&intent_id()).await.expect("読める");
    assert_eq!(found.version(), 3, "replay 後に最後の seq_nr を載せる");
    assert_eq!(found.seq_nr(), 3);
    assert_eq!(found.state(), expected.state(), "16 属性が一致する");
}

// ---------------------------------------------------------------------------
// 破損 (security-design §2 — 行を直接壊してしか作れない状態)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_journal_without_a_snapshot_is_corrupt_not_missing() {
    let fixture = Fixture::new();
    let repository = fixture.repository();
    seed(&repository).await;
    fixture
        .raw()
        .execute("DELETE FROM snapshot", [])
        .expect("スナップショットを消す");

    let err = repository
        .find_by_id(&intent_id())
        .await
        .expect_err("ジャーナルはあるのに写しが無い");
    assert_eq!(
        err,
        RepositoryError::Corrupt {
            aggregate_id: intent_id(),
            seq_nr: None,
            cause: CorruptCause::MissingSnapshot,
        }
    );
}

#[tokio::test]
async fn a_tampered_snapshot_payload_is_corrupt() {
    let fixture = Fixture::new();
    let repository = fixture.repository();
    seed(&repository).await;
    fixture
        .raw()
        .execute("UPDATE snapshot SET payload = '{\"intent_id\": 1}'", [])
        .expect("payload を改竄する");

    let err = repository
        .find_by_id(&intent_id())
        .await
        .expect_err("復号できない");
    assert!(
        matches!(
            err,
            RepositoryError::Corrupt {
                cause: CorruptCause::UndecodablePayload,
                ..
            }
        ),
        "実際: {err:?}"
    );
}

#[tokio::test]
async fn a_snapshot_written_by_another_wire_version_is_corrupt() {
    let fixture = Fixture::new();
    let repository = fixture.repository();
    seed(&repository).await;
    fixture
        .raw()
        .execute("UPDATE snapshot SET schema_version = 2", [])
        .expect("版を壊す");

    let err = repository
        .find_by_id(&intent_id())
        .await
        .expect_err("対応外の版");
    assert!(
        matches!(
            err,
            RepositoryError::Corrupt {
                cause: CorruptCause::SchemaVersion,
                ..
            }
        ),
        "実際: {err:?}"
    );
}

#[tokio::test]
async fn a_journal_row_with_an_unknown_event_type_is_corrupt() {
    let fixture = Fixture::new();
    let repository = fixture.repository();
    seed(&repository).await;

    let genesis_payload = {
        let other = Fixture::new();
        let other_repository = other.repository();
        let (aggregate, event) = genesis();
        other_repository
            .store(&event, &aggregate)
            .await
            .expect("genesis");
        snapshot_payload(&other.raw())
    };
    let conn = fixture.raw();
    rewind_snapshot_to_genesis(&conn, &genesis_payload);
    conn.execute(
        "UPDATE journal SET event_type = 'Exploded' WHERE seq_nr = 2",
        [],
    )
    .expect("変種名を壊す");

    let err = repository
        .find_by_id(&intent_id())
        .await
        .expect_err("12 語の閉集合の外");
    assert!(
        matches!(
            err,
            RepositoryError::Corrupt {
                cause: CorruptCause::UnknownEventType,
                ..
            }
        ),
        "実際: {err:?}"
    );
}

#[tokio::test]
async fn a_gap_in_the_replayed_journal_is_corrupt() {
    let fixture = Fixture::new();
    let repository = fixture.repository();
    seed(&repository).await;

    let genesis_payload = {
        let other = Fixture::new();
        let other_repository = other.repository();
        let (aggregate, event) = genesis();
        other_repository
            .store(&event, &aggregate)
            .await
            .expect("genesis");
        snapshot_payload(&other.raw())
    };
    let conn = fixture.raw();
    rewind_snapshot_to_genesis(&conn, &genesis_payload);
    conn.execute("DELETE FROM journal WHERE seq_nr = 2", [])
        .expect("途中の行を消す");

    let err = repository
        .find_by_id(&intent_id())
        .await
        .expect_err("seq_nr が飛ぶ");
    assert!(
        matches!(
            err,
            RepositoryError::Corrupt {
                cause: CorruptCause::SequenceGap,
                ..
            }
        ),
        "実際: {err:?}"
    );
}

// ---------------------------------------------------------------------------
// 前提検査 (BR1.3)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_stale_aggregate_is_refused_before_the_transaction_opens() {
    let fixture = Fixture::new();
    let repository = fixture.repository();
    let (aggregate, event) = genesis();
    repository.store(&event, &aggregate).await.expect("genesis");

    // 版を載せ替えないまま次のイベントを書こうとする (呼出側のバグ)。
    let mut stale = aggregate;
    let next = stale.complete_stage(AT).expect("索引 0 は非ゲート");
    let err = repository
        .store(&next, &stale)
        .await
        .expect_err("版 0 のまま seq_nr 2 は書けない");
    assert_eq!(
        err,
        RepositoryError::Corrupt {
            aggregate_id: intent_id(),
            seq_nr: Some(2),
            cause: CorruptCause::SequenceGap,
        }
    );

    let journal_rows: i64 = fixture
        .raw()
        .query_row("SELECT count(*) FROM journal", [], |row| row.get(0))
        .expect("件数");
    assert_eq!(journal_rows, 1, "Tx を開く前に弾くので行は増えない");
}

#[tokio::test]
async fn an_event_of_another_aggregate_is_refused() {
    let fixture = Fixture::new();
    let repository = fixture.repository();
    let (aggregate, event) = genesis();
    let foreign = WorkflowExecutionEvent::new(
        absent_intent_id(),
        event.seq_nr(),
        AT,
        core_domain::orchestration::WorkflowExecutionEventPayload::Unparked,
    );
    let err = repository
        .store(&foreign, &aggregate)
        .await
        .expect_err("別集約のイベントは書けない");
    assert!(
        matches!(
            err,
            RepositoryError::Corrupt {
                cause: CorruptCause::SequenceGap,
                ..
            }
        ),
        "実際: {err:?}"
    );
}
