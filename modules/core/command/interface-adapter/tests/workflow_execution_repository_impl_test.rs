//! `WorkflowExecutionRepositoryImpl` の実装固有の契約 (BR1.2 / BR1.3)。
//!
//! ポートの面から見える約束は `workflow_execution_repository_contract.rs` が 2 つの
//! バックエンドで共有して検査する。本ファイルが持つのは**行を直接壊してしか作れない状態**の
//! 振る舞いと、スナップショットより後ろのイベントを replay する経路である。破壊は生の SQL で
//! 行う — 実装に破壊用のフックを開けない (BR2.8)。
//!
//! 触っているのは**本家 event-store-adapter-rs の表** (`journal` / `snapshot`) である。
//! 列の並びは `journal_reader_impl.rs` のスキーマガードテストがピン留めしている。

// テストコードでは unwrap / expect を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use core_command_domain::orchestration::{
    IntentId, StageCompleted, WorkflowExecution, WorkflowExecutionEvent,
};
use core_command_domain::workflow_definition::StageSlug;
use core_command_domain::workspace::{SpaceName, StorePath};
use core_command_interface_adapter::orchestration::WorkflowExecutionRepositoryImpl;
use event_store_adapter_rs::EventStoreForSqlite;
use event_store_adapter_rs::event_envelope::EventEnvelope;
use event_store_adapter_rs::types::EventStore;

use core_command_use_case::orchestration::{
    CorruptCause, RehydratedWorkflowExecution, RepositoryError, WorkflowExecutionRepository,
};
use rusqlite::Connection;
use tempfile::TempDir;

use support::{absent_intent_id, advance, at, contract, genesis, intent_id};

/// 我々が封筒に書く型判別子 (アダプタの `EVENT_MANIFEST` と同じ綴り)。
const MANIFEST: &str = "workflow-execution-event/1";

/// 未永続の集約が提示する版。
const UNPERSISTED: usize = <Repository as WorkflowExecutionRepository>::UNPERSISTED_VERSION;

/// 本家の SQLite イベントストア (Repository が内包しているものと同じ型)。
type UpstreamStore = EventStoreForSqlite<IntentId, WorkflowExecution, WorkflowExecutionEvent>;

/// Repository の具体型 (SQLite バックエンド)。
type Repository = WorkflowExecutionRepositoryImpl<UpstreamStore>;

/// 一時ディレクトリ配下の SQLite ストアと、それを開く Repository。
struct Fixture {
    _dir: TempDir,
    path: StorePath,
}

impl Fixture {
    fn new() -> Fixture {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        let path = StorePath::for_space(&dir.path().join("aidlc"), &SpaceName::default());
        std::fs::create_dir_all(path.as_path().parent().expect("親 dir を持つ"))
            .expect("intents/ を先に作る");
        Fixture { _dir: dir, path }
    }

    fn repository(&self) -> Repository {
        WorkflowExecutionRepositoryImpl::open(&self.path).expect("ストアは開ける")
    }

    fn raw(&self) -> Connection {
        Connection::open(self.path.as_path()).expect("生の接続")
    }

    /// Repository を経由せずに同じストアへ書くためのハンドル
    /// (ジャーナルだけの追記でスナップショットとずらす唯一の口)。
    fn store(&self) -> UpstreamStore {
        UpstreamStore::new(self.path.as_path()).expect("本家ストアは開ける")
    }
}

/// genesis + 2 コマンドを書き、最後の再水和結果を返す。
async fn seed(repository: &mut Repository) -> RehydratedWorkflowExecution {
    let held = support::store_genesis(repository).await;
    let held = advance(repository, &held, |aggregate| {
        aggregate.complete_stage(at())
    })
    .await;
    advance(repository, &held, |aggregate| {
        aggregate.open_gate(vec!["intent.md".to_string()], at())
    })
    .await
}

/// スナップショットの写しを genesis 直後の姿へ巻き戻す (replay 経路を作る唯一の手)。
///
/// 版の列は触らない — 巻き戻すのは「写しが古い」状態であって、楽観 version ではない。
/// `seq_nr` 列は写しと同じ時点を指すので一緒に戻す (v3 は列を実値で持つ)。
fn rewind_snapshot_to_genesis(conn: &Connection, payload: &[u8]) {
    conn.execute("UPDATE snapshot SET payload = ?1, seq_nr = 1", [payload])
        .expect("スナップショットを巻き戻す");
}

/// 現在のスナップショット payload。
fn snapshot_payload(conn: &Connection) -> Vec<u8> {
    conn.query_row("SELECT payload FROM snapshot", [], |row| row.get(0))
        .expect("スナップショット payload")
}

/// 別のストアに genesis だけを書いて、その写しの payload を借りてくる。
async fn genesis_payload() -> Vec<u8> {
    let other = Fixture::new();
    let mut repository = other.repository();
    support::store_genesis(&mut repository).await;
    snapshot_payload(&other.raw())
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
async fn the_version_after_a_read_without_replay_is_the_one_the_store_assigned() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    let expected = seed(&mut repository).await;

    let found = repository.find_by_id(&intent_id()).await.expect("読める");
    assert_eq!(found.version(), 3, "3 回の書込ぶん採番されている");
    assert_eq!(found.aggregate().seq_nr(), 3);
    assert_eq!(found.aggregate().state(), expected.aggregate().state());
}

#[tokio::test]
async fn a_replay_does_not_move_the_version_the_store_assigned() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    let expected = seed(&mut repository).await;

    // 写しだけを genesis 直後へ巻き戻すと、seq_nr 2〜3 が replay 経路を通る。
    rewind_snapshot_to_genesis(&fixture.raw(), &genesis_payload().await);

    let found = repository.find_by_id(&intent_id()).await.expect("読める");
    assert_eq!(found.aggregate().seq_nr(), 3, "replay で追いつく");
    assert_eq!(
        found.aggregate().state(),
        expected.aggregate().state(),
        "16 属性が一致する"
    );
    assert_eq!(
        found.version(),
        3,
        "楽観 version は列の値のまま — replay では動かない (BR5.3)"
    );
}

// ---------------------------------------------------------------------------
// 破損 (security-design §2 — 行を直接壊してしか作れない状態)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_journal_without_a_snapshot_is_corrupt_not_missing() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    seed(&mut repository).await;
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
    let mut repository = fixture.repository();
    seed(&mut repository).await;
    fixture
        .raw()
        .execute("UPDATE snapshot SET payload = X'7B226964223A317D'", [])
        .expect("payload を改竄する");

    let err = repository
        .find_by_id(&intent_id())
        .await
        .expect_err("復号できない");
    assert_eq!(
        err,
        RepositoryError::Corrupt {
            aggregate_id: intent_id(),
            seq_nr: None,
            cause: CorruptCause::UndecodablePayload,
        }
    );
}

#[tokio::test]
async fn a_snapshot_that_breaks_an_aggregate_invariant_is_refused_by_the_decoder() {
    // オーナー裁定 2026-08-27 (A) の**通し確認**: 集約の serde は memento 経由なので、
    // 復号は `from_state` の検査点を通る。JSON としては読めるが不変条件を破る行 —
    // ここでは範囲外カーソル — が、ストア越しでも黙って通らないことを固定する。
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    seed(&mut repository).await;
    let conn = fixture.raw();
    let before = String::from_utf8(snapshot_payload(&conn)).expect("payload は UTF-8 の JSON");
    assert!(before.contains(r#""cursor":1"#), "{before}");
    conn.execute(
        r#"UPDATE snapshot SET payload = CAST(replace(CAST(payload AS TEXT), '"cursor":1', '"cursor":99') AS BLOB)"#,
        [],
    )
    .expect("カーソルを範囲外へ");

    let err = repository
        .find_by_id(&intent_id())
        .await
        .expect_err("不変条件を破る写しは復号できない");
    assert_eq!(
        err,
        RepositoryError::Corrupt {
            aggregate_id: intent_id(),
            seq_nr: None,
            cause: CorruptCause::UndecodablePayload,
        }
    );
}

#[tokio::test]
async fn a_journal_row_with_an_unknown_event_type_is_corrupt() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    seed(&mut repository).await;

    let conn = fixture.raw();
    rewind_snapshot_to_genesis(&conn, &genesis_payload().await);
    conn.execute(
        r#"UPDATE journal
           SET payload = CAST(replace(CAST(payload AS TEXT), '"StageCompleted"', '"Exploded"') AS BLOB)
           WHERE seq_nr = 2"#,
        [],
    )
    .expect("変種名を壊す");

    let err = repository
        .find_by_id(&intent_id())
        .await
        .expect_err("12 語の閉集合の外");
    assert_eq!(
        err,
        RepositoryError::Corrupt {
            aggregate_id: intent_id(),
            seq_nr: None,
            cause: CorruptCause::UndecodablePayload,
        }
    );
}

#[tokio::test]
async fn a_gap_in_the_replayed_journal_is_corrupt() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    seed(&mut repository).await;

    let conn = fixture.raw();
    rewind_snapshot_to_genesis(&conn, &genesis_payload().await);
    conn.execute("DELETE FROM journal WHERE seq_nr = 2", [])
        .expect("途中の行を消す");

    let err = repository
        .find_by_id(&intent_id())
        .await
        .expect_err("seq_nr が飛ぶ");
    assert_eq!(
        err,
        RepositoryError::Corrupt {
            aggregate_id: intent_id(),
            seq_nr: Some(3),
            cause: CorruptCause::SequenceGap,
        }
    );
}

#[tokio::test]
async fn a_replayed_event_naming_a_stage_outside_the_plan_is_corrupt() {
    // 復号はできるが、解決済み計画に無いステージを名指すイベント。`apply_event` の
    // `UnknownStage` は `Corrupt(InvariantViolation)` へ写す (SequenceGap とは別の原因)。
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    let (aggregate, event) = genesis();
    repository
        .store(&event, &aggregate, UNPERSISTED)
        .await
        .expect("genesis");

    let mut store = fixture.store();
    let bogus = EventEnvelope::new(
        intent_id(),
        2,
        at(),
        WorkflowExecutionEvent::StageCompleted(StageCompleted::new(
            StageSlug::parse("no-such-stage").expect("文法内の slug"),
            None,
        )),
    )
    .with_manifest(MANIFEST);
    store
        .persist_event(bogus, 1)
        .await
        .expect("ジャーナルだけに追記");

    let err = repository
        .find_by_id(&intent_id())
        .await
        .expect_err("計画に無いステージ");
    assert_eq!(
        err,
        RepositoryError::Corrupt {
            aggregate_id: intent_id(),
            seq_nr: Some(2),
            cause: CorruptCause::InvariantViolation,
        }
    );
}

// ---------------------------------------------------------------------------
// 場所 (失敗の材料)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_sqlite_repository_knows_where_it_writes() {
    let fixture = Fixture::new();
    let repository = fixture.repository();
    assert_eq!(repository.path(), Some(&fixture.path));
}

#[tokio::test]
async fn opening_under_a_missing_parent_directory_is_a_not_found() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    // `intents/` を作らずに開く (upstream の既存ディレクトリなので我々は作らない — BR2.1)。
    let path = StorePath::for_space(&dir.path().join("aidlc"), &SpaceName::default());
    let err = WorkflowExecutionRepositoryImpl::open(&path).expect_err("親 dir が無い");
    assert_eq!(
        err,
        RepositoryError::Io {
            kind: std::io::ErrorKind::NotFound,
            path: Some(path.as_path().to_path_buf()),
        }
    );
}

/// ストアの表ごと消えた場合、失敗の材料にはファイルの場所が載る (監査 C24)。
#[tokio::test]
async fn a_broken_store_reports_the_file_it_was_reading() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    seed(&mut repository).await;
    fixture
        .raw()
        .execute_batch("DROP TABLE snapshot; DROP TABLE journal")
        .expect("表ごと落とす");

    let err = repository
        .find_by_id(&intent_id())
        .await
        .expect_err("表が無い");
    assert_eq!(
        err,
        RepositoryError::Io {
            kind: std::io::ErrorKind::Other,
            path: Some(fixture.path.as_path().to_path_buf()),
        }
    );
}

/// スナップショットだけを読める状態でジャーナルが読めないと、`NotFound` の判定にも失敗する。
#[tokio::test]
async fn a_missing_journal_table_fails_the_not_found_check() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    seed(&mut repository).await;
    fixture
        .raw()
        .execute_batch("DELETE FROM snapshot; DROP TABLE journal")
        .expect("写しを消してジャーナルを落とす");

    let err = repository
        .find_by_id(&intent_id())
        .await
        .expect_err("ジャーナルを読めない");
    assert_eq!(
        err,
        RepositoryError::Io {
            kind: std::io::ErrorKind::Other,
            path: Some(fixture.path.as_path().to_path_buf()),
        }
    );
}

// 契約テストと重ならない補助 (未使用の import を避けるための参照)。
#[tokio::test]
async fn the_contract_seed_writes_five_events() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    let held = contract::seed(&mut repository).await;
    assert_eq!(held.aggregate().seq_nr(), 5);
    assert_eq!(held.version(), 5);
}

#[tokio::test]
async fn a_journal_row_with_a_foreign_manifest_is_refused_before_replay() {
    // 本家は manifest を検証せず復号して返す。読取側 (JournalReaderImpl) と同じ拒否条件で、
    // 再生経路 (find_by_id) も foreign manifest を状態遷移に流さない (PR #31 CodeRabbit 指摘)。
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    let (aggregate, started) = genesis();
    repository
        .store(&started, &aggregate, UNPERSISTED)
        .await
        .expect("genesis は書ける");
    let held = repository
        .find_by_id(&intent_id())
        .await
        .expect("握り直せる");
    advance(&mut repository, &held, |aggregate| {
        aggregate.complete_stage(at())
    })
    .await;

    // 更新は常にスナップショット同時書込なので、通常の store では journal 再生が空になる。
    // 写しを genesis へ巻き戻して「スナップショットの先に journal 行がある」状態を作り、
    // その行に別の型判別子を名乗らせる。
    let conn = fixture.raw();
    let payload = genesis_payload().await;
    rewind_snapshot_to_genesis(&conn, &payload);
    conn.execute(
        "UPDATE journal SET manifest = 'foreign-type/9' WHERE seq_nr = 2",
        [],
    )
    .expect("2 件目の行に別の型判別子を名乗らせる");
    drop(conn);

    let error = repository
        .find_by_id(&intent_id())
        .await
        .expect_err("foreign manifest は再生前に拒否される");
    assert!(
        matches!(
            &error,
            RepositoryError::Corrupt {
                seq_nr: Some(2),
                cause: CorruptCause::UndecodablePayload,
                ..
            }
        ),
        "実際: {error:?}"
    );
}
