//! `IntentExecutionRepositoryImpl` の実装固有の契約 (BR1.2 / BR1.3)。
//!
//! ポートの面から見える約束は `intent_repository_contract.rs` が 2 つの
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

use core_command_domain::orchestration::{IntentExecution, IntentExecutionEvent, StageCompleted};
use core_command_domain::workflow_definition::StageSlug;
use core_command_domain::workspace::{SpaceName, StorePath};
use core_command_interface_adapter::orchestration::{
    AggregateKey, IntentExecutionRepositoryImpl, IntentExecutionSqliteStore, WireEvent,
};
use event_store_adapter_rs::event_envelope::EventEnvelope;
use event_store_adapter_rs::types::EventStore;

use core_command_use_case::orchestration::{IntentExecutionRepository, RepositoryError};
use rusqlite::Connection;
use tempfile::TempDir;

use support::{absent_execution_id, advance, at, contract, execution_id, genesis, intent};

/// 我々が封筒に書く型判別子 (アダプタの `EVENT_MANIFEST` と同じ綴り)。
const MANIFEST: &str = "intent-execution-event/1";

/// 本家の SQLite イベントストア (Repository が内包しているものと同じ型)。
type UpstreamStore = IntentExecutionSqliteStore;

/// Repository の具体型 (SQLite バックエンド)。
type Repository = IntentExecutionRepositoryImpl<UpstreamStore>;

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
        IntentExecutionRepositoryImpl::open(&self.path).expect("ストアは開ける")
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
async fn seed(repository: &mut Repository) -> IntentExecution {
    let held = support::store_genesis(repository).await;
    let held = advance(repository, &held, |aggregate| {
        aggregate.complete_stage(&intent(), at())
    })
    .await;
    advance(repository, &held, |aggregate| {
        aggregate.open_gate(&intent(), vec!["intent.md".to_string()], at())
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
        .find_by_id(&absent_execution_id())
        .await
        .expect_err("書いていない集約");
    assert!(matches!(
        err,
        RepositoryError::NotFound { id } if id == absent_execution_id()
    ));
}

#[tokio::test]
async fn the_version_after_a_read_without_replay_is_the_one_the_store_assigned() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    let expected = seed(&mut repository).await;

    let found = repository
        .find_by_id(&execution_id())
        .await
        .expect("読める");
    assert_eq!(found.version(), 3, "3 回の書込ぶん採番されている");
    assert_eq!(found.seq_nr(), 3);
    assert_eq!(found, expected);
}

#[tokio::test]
async fn a_replay_does_not_move_the_version_the_store_assigned() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    let expected = seed(&mut repository).await;

    // 写しだけを genesis 直後へ巻き戻すと、seq_nr 2〜3 が replay 経路を通る。
    rewind_snapshot_to_genesis(&fixture.raw(), &genesis_payload().await);

    let found = repository
        .find_by_id(&execution_id())
        .await
        .expect("読める");
    assert_eq!(found.seq_nr(), 3, "replay で追いつく");
    assert_eq!(found, expected, "全状態が一致する");
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
        .find_by_id(&execution_id())
        .await
        .expect_err("ジャーナルはあるのに写しが無い");
    assert!(matches!(
        &err,
        RepositoryError::Corrupt { id, seq_nr: None, .. } if *id == execution_id()
    ));
    assert_eq!(
        std::error::Error::source(&err)
            .expect("原因が連鎖する")
            .to_string(),
        "missing snapshot"
    );
}

#[tokio::test]
async fn a_snapshot_alone_is_a_sufficient_rehydration_base() {
    // 差分再生の基底はスナップショットである — ジャーナル行が 1 件も無くても、基底の時点の
    // 状態へ再水和できる (issue #44。既定ストラテジでは基底は genesis なので、seed の後段は
    // 差分行と共に見えなくなるが、それはジャーナルを消した側の損失であって破損分類ではない)。
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    seed(&mut repository).await;
    fixture
        .raw()
        .execute("DELETE FROM journal", [])
        .expect("ジャーナルを空にする");

    let found = repository
        .find_by_id(&execution_id())
        .await
        .expect("基底 (genesis スナップショット) から再水和できる");
    assert_eq!(found.seq_nr(), 1, "基底の時点の状態");
    assert_eq!(found.version(), 3, "版の正本は行の列のまま");
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
        .find_by_id(&execution_id())
        .await
        .expect_err("復号できない");
    assert!(matches!(
        &err,
        RepositoryError::Corrupt { id, seq_nr: None, .. } if *id == execution_id()
    ));
    assert_eq!(
        std::error::Error::source(&err)
            .expect("原因が連鎖する")
            .to_string(),
        "store deserialization failed"
    );
}

#[tokio::test]
async fn a_tampered_snapshot_state_is_corrupt() {
    // 基底はスナップショット (= ある時点の集約) である — 形の読める改竄 (ここでは範囲外
    // カーソル) は集約の完全コンストラクタが拒み、`Corrupt` に写る (issue #44 で全再生から
    // 差分再生へ転換した帰結。BR1.5)。
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    seed(&mut repository).await;
    let conn = fixture.raw();
    // 既定ストラテジではスナップショットは genesis のまま (カーソル 0)。
    let before = String::from_utf8(snapshot_payload(&conn)).expect("payload は UTF-8 の JSON");
    assert!(before.contains(r#""cursor":0"#), "{before}");
    conn.execute(
        r#"UPDATE snapshot SET payload = CAST(replace(CAST(payload AS TEXT), '"cursor":0', '"cursor":99') AS BLOB)"#,
        [],
    )
    .expect("カーソルを範囲外へ");
    drop(conn);

    let err = repository
        .find_by_id(&execution_id())
        .await
        .expect_err("壊れた基底は Corrupt");
    assert!(matches!(
        &err,
        RepositoryError::Corrupt { id, seq_nr: None, .. } if *id == execution_id()
    ));
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
        .find_by_id(&execution_id())
        .await
        .expect_err("12 語の閉集合の外");
    assert!(matches!(
        &err,
        RepositoryError::Corrupt { id, seq_nr: None, .. } if *id == execution_id()
    ));
    assert_eq!(
        std::error::Error::source(&err)
            .expect("原因が連鎖する")
            .to_string(),
        "store deserialization failed"
    );
}

#[tokio::test]
#[should_panic(expected = "apply_event: corrupted history")]
async fn a_replayed_event_naming_a_stage_outside_the_plan_crashes_reconstruction() {
    // 復号はできるが、解決済み計画に無いステージを名指すイベント — 壊れた歴史であり、
    // 再構成は失敗を返さずクラッシュする (オーナー裁定 2026-08-30)。
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    let (aggregate, event) = genesis();
    repository.store(&event, &aggregate).await.expect("genesis");

    let mut store = fixture.store();
    // 生の行を作るので、封筒に載せるのはアダプタの永続化 DTO である。
    let bogus = EventEnvelope::new(
        AggregateKey::of(&execution_id()),
        2,
        at(),
        WireEvent::of(&IntentExecutionEvent::StageCompleted(StageCompleted::new(
            StageSlug::parse("no-such-stage").expect("文法内の slug"),
            None,
        ))),
    )
    .with_manifest(MANIFEST);
    store
        .persist_event(bogus, 1)
        .await
        .expect("ジャーナルだけに追記");

    let _ = repository.find_by_id(&execution_id()).await;
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
    let err = IntentExecutionRepositoryImpl::open(&path).expect_err("親 dir が無い");
    assert!(matches!(
        err,
        RepositoryError::Io {
            kind: std::io::ErrorKind::NotFound,
            path: Some(actual),
        } if actual == path.as_path()
    ));
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
        .find_by_id(&execution_id())
        .await
        .expect_err("表が無い");
    assert!(matches!(
        err,
        RepositoryError::Io {
            kind: std::io::ErrorKind::Other,
            path: Some(actual),
        } if actual == fixture.path.as_path()
    ));
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
        .find_by_id(&execution_id())
        .await
        .expect_err("ジャーナルを読めない");
    assert!(matches!(
        err,
        RepositoryError::Io {
            kind: std::io::ErrorKind::Other,
            path: Some(actual),
        } if actual == fixture.path.as_path()
    ));
}

// 契約テストと重ならない補助 (未使用の import を避けるための参照)。
#[tokio::test]
async fn the_contract_seed_writes_five_events() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    let held = contract::seed(&mut repository).await;
    assert_eq!(held.seq_nr(), 5);
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
        .store(&started, &aggregate)
        .await
        .expect("genesis は書ける");
    let held = repository
        .find_by_id(&execution_id())
        .await
        .expect("握り直せる");
    advance(&mut repository, &held, |aggregate| {
        aggregate.complete_stage(&intent(), at())
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
        .find_by_id(&execution_id())
        .await
        .expect_err("foreign manifest は再生前に拒否される");
    assert!(
        matches!(
            &error,
            RepositoryError::Corrupt {
                seq_nr: Some(2),
                ..
            }
        ),
        "実際: {error:?}"
    );
    assert_eq!(
        std::error::Error::source(&error)
            .expect("原因が連鎖する")
            .to_string(),
        "foreign manifest"
    );
}

#[tokio::test]
async fn a_foreign_manifest_before_the_snapshot_base_is_not_read() {
    // 差分再生は基底の通番より後の行しか読まない — genesis 行の名乗りが違っても、基底
    // (ここでは genesis スナップショット) 以後の差分行が正しければ再水和は成立する
    // (issue #44。差分行の foreign manifest は従来どおり拒否される — 別テストが固定)。
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    let expected = seed(&mut repository).await;
    let conn = fixture.raw();
    let genesis = genesis_payload().await;
    rewind_snapshot_to_genesis(&conn, &genesis);
    conn.execute(
        "UPDATE journal SET manifest = 'foreign-type/9' WHERE seq_nr = 1",
        [],
    )
    .expect("genesis 行の名乗りを差し替える");
    drop(conn);

    let found = repository
        .find_by_id(&execution_id())
        .await
        .expect("基底以後の差分だけで再水和できる");
    assert_eq!(found, expected);
}

#[tokio::test]
async fn a_snapshot_that_breaks_the_aggregate_invariants_is_corrupt() {
    // JSON としては読めるが集約不変条件を破るスナップショット (実効 SKIP のカーソル)。基底の
    // 復元は完全コンストラクタを必ず通るので、`Corrupt` に写る (BR1.5 — issue #44)。
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    seed(&mut repository).await;
    let conn = fixture.raw();
    let genesis = genesis_payload().await;
    rewind_snapshot_to_genesis(&conn, &genesis);
    conn.execute(
        r#"UPDATE snapshot SET payload = CAST(replace(CAST(payload AS TEXT), '"overlay":["Execute"', '"overlay":["Skip"') AS BLOB)"#,
        [],
    )
    .expect("先頭ステージの実効プランを SKIP に畳む");
    drop(conn);

    let err = repository
        .find_by_id(&execution_id())
        .await
        .expect_err("不変条件を破る基底は Corrupt");
    assert!(matches!(
        &err,
        RepositoryError::Corrupt { id, seq_nr: None, .. } if *id == execution_id()
    ));
}

#[tokio::test]
async fn a_replayed_row_whose_spelling_is_outside_the_closed_set_is_refused() {
    // 再生行の payload が閉集合外の綴りを名乗る場合。DTO からドメインへ写す時点で止まるので、
    // 壊れた値が状態遷移に流れ込まない。
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    seed(&mut repository).await;
    let conn = fixture.raw();
    let genesis = genesis_payload().await;
    rewind_snapshot_to_genesis(&conn, &genesis);
    conn.execute(
        r#"UPDATE journal SET payload = CAST(replace(CAST(payload AS TEXT), '"stage":"state-init"', '"stage":"Not A Slug"') AS BLOB) WHERE seq_nr = 2"#,
        [],
    )
    .expect("再生行のステージ参照を壊す");

    let err = repository
        .find_by_id(&execution_id())
        .await
        .expect_err("閉集合外の綴りは再生前に拒否される");
    assert!(matches!(
        &err,
        RepositoryError::Corrupt { id, seq_nr: Some(2), .. } if *id == execution_id()
    ));
    assert_eq!(
        std::error::Error::source(&err)
            .expect("原因が連鎖する")
            .to_string(),
        "undecodable payload"
    );
}

// ---- issue #44: スナップショットストラテジ (初回必須・N 件ごと・差分再生の一致) ----

/// snapshot 行の seq_nr 列 (基底がどこまで進んでいるか)。
fn snapshot_seq(conn: &Connection) -> i64 {
    conn.query_row("SELECT seq_nr FROM snapshot", [], |row| row.get(0))
        .expect("スナップショット行の seq_nr")
}

#[tokio::test]
async fn the_first_store_always_writes_the_snapshot_base() {
    // 初回は必ず persist_event_and_snapshot — 基底が無いとリプレイできない (本家の作成規約)。
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    let (aggregate, event) = genesis();
    repository.store(&event, &aggregate).await.expect("genesis");
    assert_eq!(snapshot_seq(&fixture.raw()), 1, "基底は genesis 時点");
}

#[tokio::test]
async fn the_strategy_refreshes_the_snapshot_every_n_events() {
    // N=2: 偶数通番の書込でだけ基底が進む。呼出側は store に任せるだけである
    // (オーナー裁定 2026-08-30、本家 example の形)。
    let fixture = Fixture::new();
    let mut repository = fixture.repository().with_snapshot_strategy(
        core_command_interface_adapter::orchestration::SnapshotStrategy::every(
            std::num::NonZeroUsize::new(2).expect("非零"),
        ),
    );
    let held = support::store_genesis(&mut repository).await;
    assert_eq!(snapshot_seq(&fixture.raw()), 1);
    let held = advance(&mut repository, &held, |aggregate| {
        aggregate.complete_stage(&intent(), at())
    })
    .await;
    assert_eq!(snapshot_seq(&fixture.raw()), 2, "seq 2 は基底を書き直す");
    let held = advance(&mut repository, &held, |aggregate| {
        aggregate.open_gate(&intent(), vec!["intent.md".to_string()], at())
    })
    .await;
    assert_eq!(snapshot_seq(&fixture.raw()), 2, "seq 3 はイベントのみ");
    assert_eq!(held.version(), 3, "イベントのみの書込でも版は進む");
    let held = advance(&mut repository, &held, |aggregate| {
        aggregate.approve_gate(&intent(), Some("ok".to_string()), at())
    })
    .await;
    assert_eq!(snapshot_seq(&fixture.raw()), 4, "seq 4 は基底を書き直す");
    assert_eq!(held.seq_nr(), 4);
}

#[tokio::test]
async fn a_stale_snapshot_plus_delta_matches_the_freshest_state() {
    // 基底 (genesis) が古いままでも、差分再生が最新状態を返す — 全再生と同じ答えになる
    // ことがストラテジ導入の受入条件である (issue #44)。
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    let expected = seed(&mut repository).await;
    assert_eq!(
        snapshot_seq(&fixture.raw()),
        1,
        "既定 N=10 では基底は genesis のまま"
    );

    let found = repository
        .find_by_id(&execution_id())
        .await
        .expect("基底 + 差分で最新へ");
    assert_eq!(found, expected);
    assert_eq!(found.seq_nr(), 3);
}

#[tokio::test]
async fn a_gap_in_the_delta_rows_is_corrupt_not_a_crash() {
    // 基底より後の行が 1 件欠けても読取はプロセスを止めない — 他の破損と同じく
    // `Corrupt` に分類する (CodeRabbit 指摘 — 差分再生への転換で到達可能になった経路)。
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    seed(&mut repository).await;
    fixture
        .raw()
        .execute("DELETE FROM journal WHERE seq_nr = 2", [])
        .expect("差分行を 1 件欠けさせる");

    let err = repository
        .find_by_id(&execution_id())
        .await
        .expect_err("行の欠けは Corrupt");
    assert!(matches!(
        &err,
        RepositoryError::Corrupt { id, seq_nr: Some(3), .. } if *id == execution_id()
    ));
    assert_eq!(
        std::error::Error::source(&err)
            .expect("原因が連鎖する")
            .to_string(),
        "sequence gap"
    );
}
