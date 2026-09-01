//! `IntentRepositoryImpl` の実装固有の契約 (issue #50)。
//!
//! ポートの面から見える約束は `intent_repository_contract.rs` が 3 つの実装で共有して
//! 検査する。本ファイルが持つのは**行を直接壊してしか作れない状態**の振る舞いと、
//! スナップショットより後ろのイベントを replay する経路、そして実行ストリームとの
//! **同一ストアファイル同居** (issue #50 の設計裁定) である。破壊は生の SQL で行う —
//! 実装に破壊用のフックを開けない (BR2.8)。

// テストコードでは unwrap / expect を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use core_command_domain::orchestration::IntentExecutionId;
use core_command_domain::workspace::{SpaceName, StorePath};
use core_command_interface_adapter::orchestration::{
    IntentAggregateKeyDto, IntentEventDto, IntentExecutionRepositoryImpl, IntentRepositoryImpl,
    IntentSqliteStore,
};
use event_store_adapter_rs::event_envelope::EventEnvelope;
use event_store_adapter_rs::types::EventStore;

use core_command_use_case::orchestration::{
    IntentExecutionRepository, IntentRepository, RepositoryError,
};
use rusqlite::Connection;
use tempfile::TempDir;

use support::{
    EXECUTION, INTENT, at, genesis_for, intent_genesis, intent_id, store_intent_genesis,
};

/// 我々が封筒に書く型判別子 (アダプタの `EVENT_MANIFEST` と同じ綴り)。
const MANIFEST: &str = "intent-event/1";

/// Repository の具体型 (SQLite バックエンド)。
type Repository = IntentRepositoryImpl<IntentSqliteStore>;

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
        IntentRepositoryImpl::open(&self.path).expect("ストアは開ける")
    }

    fn raw(&self) -> Connection {
        Connection::open(self.path.as_path()).expect("生の接続")
    }

    /// Repository を経由せずに同じストアへ書くためのハンドル
    /// (スナップショットより後ろへジャーナル行を足す唯一の口)。
    fn store(&self) -> IntentSqliteStore {
        IntentSqliteStore::new(self.path.as_path()).expect("本家ストアは開ける")
    }
}

/// スナップショットより後ろの差分行 (`seq_nr = 2`) を 1 件書く。
///
/// intent のイベントは現状 `Created` 1 種なので、差分の中身も誕生記録である — 差分適用は
/// 何も変えない (`apply_event` の genesis 腕は no-op)。replay の**経路**を通すのが目的で
/// ある。
async fn append_delta(fixture: &Fixture, seq_nr: usize, manifest: &str) {
    let (_, event) = intent_genesis();
    let envelope = EventEnvelope::new(
        IntentAggregateKeyDto::of(&intent_id()),
        seq_nr,
        at(),
        IntentEventDto::of(&event),
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
        "SELECT payload FROM journal WHERE seq_nr = ?1",
        [seq_nr],
        |row| row.get(0),
    )
    .expect("ジャーナル payload")
}

// ---------------------------------------------------------------------------
// 同一ストアファイル同居 (issue #50 の設計裁定)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_intent_stream_coexists_with_the_execution_stream_in_the_same_file() {
    let fixture = Fixture::new();

    // 識別子は別の UUID (実運用と同じ形)。本家の journal は `(aid, seq_nr)` に UNIQUE 索引を
    // **type_name 抜きの生値**で張るため、同居の前提は識別子の値の一意性である
    // (`IntentAggregateKeyDto` の doc 参照)。UUID どうしなら満たされる。
    let mut intent_execution_repository =
        IntentExecutionRepositoryImpl::open(&fixture.path).expect("実行ストアは同じファイル");
    let (execution, started) = genesis_for(IntentExecutionId::parse(EXECUTION).unwrap());
    intent_execution_repository
        .store(&started, &execution)
        .await
        .expect("実行の genesis");

    let mut intent_repository = fixture.repository();
    let stored = store_intent_genesis(&mut intent_repository).await;

    assert_eq!(
        intent_repository
            .find_by_id(&intent_id())
            .await
            .expect("intent は読める"),
        stored
    );
    assert_eq!(
        intent_execution_repository
            .find_by_id(execution.id())
            .await
            .expect("実行も読める")
            .seq_nr(),
        1,
        "同じファイルに同居してもストリームは混ざらない (前提は識別子の値の一意性)"
    );
}

// ---------------------------------------------------------------------------
// 差分再生の経路 (本家 example 同型)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_rehydration_replays_the_delta_beyond_the_snapshot() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    let stored = store_intent_genesis(&mut repository).await;

    append_delta(&fixture, 2, MANIFEST).await;

    let found = repository.find_by_id(&intent_id()).await.expect("読める");
    assert_eq!(
        found, stored,
        "差分 (genesis 腕は no-op) を適用しても状態は誕生の材料のまま"
    );
}

#[tokio::test]
async fn a_gap_in_the_delta_is_corrupt() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    store_intent_genesis(&mut repository).await;

    // seq_nr = 2 を飛ばして 3 を書く (本家の CAS は version しか見ないので追記はできる)。
    append_delta(&fixture, 3, MANIFEST).await;

    let err = repository
        .find_by_id(&intent_id())
        .await
        .expect_err("行の欠け");
    assert!(matches!(
        &err,
        RepositoryError::Corrupt { id, seq_nr: Some(3), .. } if *id == intent_id()
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
    store_intent_genesis(&mut repository).await;

    // 実行ジャーナルの判別子を名乗る行 — 別の型名・別の読み方の版は状態遷移に流さない。
    append_delta(&fixture, 2, "intent-execution-event/1").await;

    let err = repository
        .find_by_id(&intent_id())
        .await
        .expect_err("foreign manifest");
    assert!(matches!(
        &err,
        RepositoryError::Corrupt { id, seq_nr: Some(2), .. } if *id == intent_id()
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
    store_intent_genesis(&mut repository).await;
    append_delta(&fixture, 2, MANIFEST).await;

    // 形 (DTO) としては読めるが、識別子の文法違反でドメインへ戻せない行にする。
    let conn = fixture.raw();
    let payload = journal_payload(&conn, 2);
    let broken = String::from_utf8(payload)
        .expect("payload は JSON 文字列")
        .replace(INTENT, "not-a-uuid");
    conn.execute(
        "UPDATE journal SET payload = ?1 WHERE seq_nr = 2",
        [broken.into_bytes()],
    )
    .expect("payload を壊す");

    let err = repository
        .find_by_id(&intent_id())
        .await
        .expect_err("復号不能");
    assert!(matches!(
        &err,
        RepositoryError::Corrupt { id, seq_nr: Some(2), .. } if *id == intent_id()
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
    store_intent_genesis(&mut repository).await;

    fixture
        .raw()
        .execute("DELETE FROM snapshot", [])
        .expect("スナップショット行を消す");

    let err = repository
        .find_by_id(&intent_id())
        .await
        .expect_err("片方だけは矛盾");
    assert!(matches!(
        &err,
        RepositoryError::Corrupt { id, seq_nr: None, .. } if *id == intent_id()
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
    store_intent_genesis(&mut repository).await;

    fixture
        .raw()
        .execute("UPDATE snapshot SET payload = X'00'", [])
        .expect("payload をバイトごと壊す");

    let err = repository
        .find_by_id(&intent_id())
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
    store_intent_genesis(&mut repository).await;

    let conn = fixture.raw();
    let payload: Vec<u8> = conn
        .query_row("SELECT payload FROM snapshot", [], |row| row.get(0))
        .expect("スナップショット payload");
    let broken = String::from_utf8(payload)
        .expect("payload は JSON 文字列")
        .replace(INTENT, "not-a-uuid");
    conn.execute("UPDATE snapshot SET payload = ?1", [broken.into_bytes()])
        .expect("payload を壊す");

    let err = repository
        .find_by_id(&intent_id())
        .await
        .expect_err("検査付き再構成が拒む");
    assert!(matches!(
        &err,
        RepositoryError::Corrupt { id, seq_nr: None, .. } if *id == intent_id()
    ));
    assert_eq!(
        std::error::Error::source(&err)
            .expect("原因が連鎖する")
            .to_string(),
        "undecodable payload"
    );
}

// ---------------------------------------------------------------------------
// 開けない場所
// ---------------------------------------------------------------------------

#[test]
fn opening_under_a_missing_parent_directory_is_a_not_found() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    // `intents/` を作らない — upstream の既存ディレクトリが無い状態。
    let path = StorePath::for_space(&dir.path().join("aidlc"), &SpaceName::default());
    let err = IntentRepositoryImpl::open(&path).expect_err("親 dir が無ければ開けない");
    assert!(matches!(
        err,
        RepositoryError::Io {
            kind: std::io::ErrorKind::NotFound,
            path: Some(_)
        }
    ));
}
