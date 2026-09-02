//! `WorkflowDefinitionRepositoryImpl` の実装固有の契約 (2026-08-31 の ES 転換)。
//!
//! ポートの面から見える約束は `workflow_definition_repository_contract.rs` が 2 つの
//! バックエンドで共有して検査する。本ファイルが持つのは**行を直接壊してしか作れない状態**の
//! 振る舞いと、実行・intent ストリームとの**同一ストアファイル同居**である。破壊は生の SQL で
//! 行う — 実装に破壊用のフックを開けない (BR2.8)。
//!
//! 形は `intent_repository_impl_test.rs` と同型である。定義の Repository がイベントストア形に
//! なったからこそ、同じ破損の分類 (`MissingSnapshot` / `ForeignManifest` / `SequenceGap` /
//! `Undecodable` / `StoreDeserialization`) が意味を持つようになった。

// テストコードでは unwrap / expect を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
// panic! は想定外バリアントの即時失敗という検証用途で使う。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use std::num::NonZeroUsize;

use core_command_domain::orchestration::IntentExecutionId;
use core_command_domain::workflow_definition::{WorkflowDefinition, WorkflowDefinitionEvent};
use core_command_domain::workspace::{SpaceName, StorePath};
use core_command_interface_adapter::orchestration::{
    IntentExecutionRepositoryImpl, IntentRepositoryImpl, SnapshotStrategy,
    WorkflowDefinitionAggregateKeyDto, WorkflowDefinitionEventDto,
    WorkflowDefinitionRepositoryImpl, WorkflowDefinitionSqliteStore,
};
use core_command_use_case::orchestration::{
    IntentExecutionRepository, IntentRepository, RepositoryError, WorkflowDefinitionRepository,
};
use event_store_adapter_rs::event_envelope::EventEnvelope;
use event_store_adapter_rs::types::EventStore;
use rusqlite::Connection;
use tempfile::TempDir;

use support::{
    EXECUTION, at, definition_bundle, definition_genesis, definition_id, genesis_for,
    intent_genesis, store_definition_genesis,
};

/// 我々が封筒に書く型判別子 (アダプタの `EVENT_MANIFEST` と同じ綴り)。
const MANIFEST: &str = "workflow-definition-event/1";

/// Repository の具体型 (SQLite バックエンド)。
type Repository = WorkflowDefinitionRepositoryImpl<WorkflowDefinitionSqliteStore>;

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
        WorkflowDefinitionRepositoryImpl::open(&self.path).expect("ストアは開ける")
    }

    fn raw(&self) -> Connection {
        Connection::open(self.path.as_path()).expect("生の接続")
    }

    /// Repository を経由せずに同じストアへ書くためのハンドル
    /// (スナップショットより後ろへジャーナル行を足す唯一の口)。
    fn store(&self) -> WorkflowDefinitionSqliteStore {
        WorkflowDefinitionSqliteStore::new(self.path.as_path()).expect("本家ストアは開ける")
    }
}

/// 改訂 (`Redefined`) の差分イベント。
fn redefinition() -> WorkflowDefinitionEvent {
    let mut definition = definition_genesis().0;
    definition
        .redefine(&definition_bundle(5), at())
        .expect("内容版が違えば改訂できる")
}

/// スナップショットより後ろの差分行を 1 件書く。
async fn append_delta(
    fixture: &Fixture,
    seq_nr: usize,
    manifest: &str,
    event: &WorkflowDefinitionEvent,
) {
    let envelope = EventEnvelope::new(
        WorkflowDefinitionAggregateKeyDto::of(&definition_id()),
        seq_nr,
        at(),
        WorkflowDefinitionEventDto::of(event),
    )
    .with_manifest(manifest);
    fixture
        .store()
        .persist_event(envelope, 1)
        .await
        .expect("差分行は追記できる");
}

/// 指定した通番のジャーナル行 payload。
fn journal_payload(conn: &Connection, seq_nr: i64) -> Vec<u8> {
    conn.query_row(
        "SELECT payload FROM journal WHERE seq_nr = ?1 AND aid = 'claude'",
        [seq_nr],
        |row| row.get(0),
    )
    .expect("ジャーナル payload")
}

// ---------------------------------------------------------------------------
// 同一ストアファイル同居 (3 ストリーム)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_definition_stream_coexists_with_the_other_two_in_the_same_file() {
    let fixture = Fixture::new();

    // 定義 id はハーネス名 (`claude`)、他の 2 つは UUID。本家の journal は `(aid, seq_nr)` に
    // UNIQUE 索引を **type_name 抜きの生値**で張るため、同居の前提は識別子の値の一意性で
    // ある — ハーネス名と UUID は決して衝突しない。
    let mut intent_repository =
        IntentRepositoryImpl::open(&fixture.path).expect("intent ストアは同じファイル");
    let (intent, created) = intent_genesis();
    intent_repository
        .store(&created, &intent)
        .await
        .expect("intent の genesis");

    let mut intent_execution_repository =
        IntentExecutionRepositoryImpl::open(&fixture.path).expect("実行ストアも同じファイル");
    let (execution, started) = genesis_for(IntentExecutionId::parse(EXECUTION).unwrap());
    intent_execution_repository
        .store(&started, &execution)
        .await
        .expect("実行の genesis");

    let mut workflow_definition_repository = fixture.repository();
    let stored = store_definition_genesis(&mut workflow_definition_repository).await;

    assert_eq!(
        workflow_definition_repository
            .find_by_id(&definition_id())
            .await
            .expect("定義は読める"),
        stored
    );
    assert_eq!(
        intent_repository
            .find_by_id(intent.id())
            .await
            .expect("intent も読める"),
        intent
    );
    assert_eq!(
        intent_execution_repository
            .find_by_id(execution.id())
            .await
            .expect("実行も読める")
            .seq_nr(),
        1,
        "3 ストリームが同居しても混ざらない"
    );
}

// ---------------------------------------------------------------------------
// 差分再生の経路 (本家 example 同型)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_rehydration_replays_the_delta_beyond_the_snapshot() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    store_definition_genesis(&mut repository).await;

    append_delta(&fixture, 2, MANIFEST, &redefinition()).await;

    let found = repository
        .find_by_id(&definition_id())
        .await
        .expect("読める");
    assert_eq!(
        found.revision(),
        definition_bundle(5).revision(),
        "スナップショットより後ろの改訂が適用される"
    );
    assert_eq!(found.graph().len(), 5);
    assert_eq!(found.seq_nr(), 2);
}

#[tokio::test]
async fn a_genesis_payload_in_the_delta_replays_as_a_no_op() {
    // 誕生イベントが差分に現れても状態は動かない (スナップショット種が誕生を含む)。
    // 復号の経路そのものは通るので、`Defined` の payload も読めることを押さえる。
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    let stored = store_definition_genesis(&mut repository).await;

    append_delta(&fixture, 2, MANIFEST, &definition_genesis().1).await;

    let found = repository
        .find_by_id(&definition_id())
        .await
        .expect("読める");
    assert_eq!(found.revision(), stored.revision());
    assert_eq!(found.graph().len(), stored.graph().len());
    assert_eq!(found.seq_nr(), 2, "通番だけが進む");
}

#[tokio::test]
async fn a_gap_in_the_delta_is_corrupt() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    store_definition_genesis(&mut repository).await;

    // seq_nr = 2 を飛ばして 3 を書く (本家の CAS は version しか見ないので追記はできる)。
    append_delta(&fixture, 3, MANIFEST, &redefinition()).await;

    let err = repository
        .find_by_id(&definition_id())
        .await
        .expect_err("行の欠け");
    assert!(matches!(
        &err,
        RepositoryError::Corrupt { id, seq_nr: Some(3), .. } if *id == definition_id()
    ));
    assert_eq!(
        std::error::Error::source(&err)
            .expect("原因が連鎖する")
            .to_string(),
        "sequence gap"
    );
}

#[tokio::test]
async fn a_foreign_manifest_in_the_delta_is_corrupt() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    store_definition_genesis(&mut repository).await;

    // intent ジャーナルの判別子を名乗る行 — 別の型名・別の読み方の版は状態遷移に流さない。
    append_delta(&fixture, 2, "intent-event/1", &redefinition()).await;

    let err = repository
        .find_by_id(&definition_id())
        .await
        .expect_err("foreign manifest");
    assert!(matches!(
        &err,
        RepositoryError::Corrupt { id, seq_nr: Some(2), .. } if *id == definition_id()
    ));
    assert_eq!(
        std::error::Error::source(&err)
            .expect("原因が連鎖する")
            .to_string(),
        "foreign manifest"
    );
}

#[tokio::test]
async fn an_undecodable_delta_payload_is_corrupt() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    store_definition_genesis(&mut repository).await;
    append_delta(&fixture, 2, MANIFEST, &redefinition()).await;

    // 形 (DTO) としては読めるが、内容版の文法違反でドメインへ戻せない行にする。
    let conn = fixture.raw();
    let payload = journal_payload(&conn, 2);
    let broken = String::from_utf8(payload)
        .expect("payload は JSON 文字列")
        .replace("sha256:", "nope:");
    conn.execute(
        "UPDATE journal SET payload = ?1 WHERE seq_nr = 2 AND aid = 'claude'",
        [broken.into_bytes()],
    )
    .expect("payload を壊す");

    let err = repository
        .find_by_id(&definition_id())
        .await
        .expect_err("復号不能");
    assert!(matches!(
        &err,
        RepositoryError::Corrupt { id, seq_nr: Some(2), .. } if *id == definition_id()
    ));
    assert_eq!(
        std::error::Error::source(&err)
            .expect("原因が連鎖する")
            .to_string(),
        "undecodable payload"
    );
}

// ---------------------------------------------------------------------------
// 行を直接壊してしか作れない破損
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_journal_row_without_a_snapshot_row_is_corrupt() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    store_definition_genesis(&mut repository).await;

    fixture
        .raw()
        .execute("DELETE FROM snapshot", [])
        .expect("スナップショット行を消す");

    let err = repository
        .find_by_id(&definition_id())
        .await
        .expect_err("片方だけは矛盾");
    assert!(matches!(
        &err,
        RepositoryError::Corrupt { id, seq_nr: None, .. } if *id == definition_id()
    ));
    assert_eq!(
        std::error::Error::source(&err)
            .expect("原因が連鎖する")
            .to_string(),
        "missing snapshot"
    );
}

#[tokio::test]
async fn an_unreadable_snapshot_payload_is_corrupt() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    store_definition_genesis(&mut repository).await;

    fixture
        .raw()
        .execute("UPDATE snapshot SET payload = X'00'", [])
        .expect("payload をバイトごと壊す");

    let err = repository
        .find_by_id(&definition_id())
        .await
        .expect_err("ストアの復号が失敗");
    assert!(matches!(
        &err,
        RepositoryError::Corrupt { seq_nr: None, .. }
    ));
    assert_eq!(
        std::error::Error::source(&err)
            .expect("原因が連鎖する")
            .to_string(),
        "store deserialization failed"
    );
}

#[tokio::test]
async fn a_snapshot_that_decodes_but_breaks_the_domain_is_corrupt() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    store_definition_genesis(&mut repository).await;

    let conn = fixture.raw();
    let payload: Vec<u8> = conn
        .query_row("SELECT payload FROM snapshot", [], |row| row.get(0))
        .expect("スナップショット payload");
    let broken = String::from_utf8(payload)
        .expect("payload は JSON 文字列")
        .replace("sha256:", "nope:");
    conn.execute("UPDATE snapshot SET payload = ?1", [broken.into_bytes()])
        .expect("payload を壊す");

    let err = repository
        .find_by_id(&definition_id())
        .await
        .expect_err("検査付き再構成が拒む");
    assert!(matches!(
        &err,
        RepositoryError::Corrupt { id, seq_nr: None, .. } if *id == definition_id()
    ));
    assert_eq!(
        std::error::Error::source(&err)
            .expect("原因が連鎖する")
            .to_string(),
        "undecodable payload"
    );
}

// ---------------------------------------------------------------------------
// スナップショットの書き直し間隔 (実装の内部設定 — ポート面に現れない)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_snapshot_strategy_of_one_rewrites_the_snapshot_on_every_event() {
    let fixture = Fixture::new();
    let mut repository = fixture
        .repository()
        .with_snapshot_strategy(SnapshotStrategy::every(NonZeroUsize::MIN));
    let mut held = store_definition_genesis(&mut repository).await;

    let event = held
        .redefine(&definition_bundle(5), at())
        .expect("改訂できる");
    repository.store(&event, &held).await.expect("改訂は書ける");

    // スナップショット行の通番が改訂まで進んでいる = 差分ゼロで再水和できる。
    let snapshot_seq: i64 = fixture
        .raw()
        .query_row(
            "SELECT seq_nr FROM snapshot WHERE aid = 'claude'",
            [],
            |row| row.get(0),
        )
        .expect("スナップショット行");
    assert_eq!(snapshot_seq, 2, "毎イベントで書き直す設定なので基底が進む");

    let found: WorkflowDefinition = repository
        .find_by_id(&definition_id())
        .await
        .expect("読める");
    assert_eq!(found.revision(), definition_bundle(5).revision());
    assert_eq!(found.seq_nr(), 2);
}

// ---------------------------------------------------------------------------
// 開けない場所
// ---------------------------------------------------------------------------

#[test]
fn opening_under_a_missing_parent_directory_is_a_not_found() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    // `intents/` を作らない — upstream の既存ディレクトリが無い状態。
    let path = StorePath::for_space(&dir.path().join("aidlc"), &SpaceName::default());
    let err = WorkflowDefinitionRepositoryImpl::open(&path).expect_err("親 dir が無ければ開けない");
    assert!(matches!(
        err,
        RepositoryError::Io {
            kind: std::io::ErrorKind::NotFound,
            path: Some(_)
        }
    ));
}

#[tokio::test]
async fn a_sqlite_repository_reports_its_location_in_its_failures() {
    // 揮発のストアと違い、SQLite のストアは失敗の材料に場所を添えられる。
    let fixture = Fixture::new();
    let repository = fixture.repository();
    assert_eq!(repository.path(), Some(&fixture.path));

    fixture
        .raw()
        .execute("DROP TABLE journal", [])
        .expect("表ごと落とす");
    let err = repository
        .find_by_id(&definition_id())
        .await
        .expect_err("表が無ければ読めない");
    let RepositoryError::Io { path, .. } = err else {
        panic!("Io を期待した: {err:?}");
    };
    assert_eq!(path.as_deref(), Some(fixture.path.as_path()));
}
