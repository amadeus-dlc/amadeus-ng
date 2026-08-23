//! クラッシュ再構成 (BR5.2 (a)) — 書込のあとプロセスが落ちたと見なし、新しい接続で
//! 同じ集約が同じ状態に戻ることを固定する。
//!
//! 「落ちた」は接続 (と Repository) を drop することで表す。SQLite の `COMMIT` を通った
//! 書込だけが残り、途中で捨てられた Tx は残らない — その 2 つを同じファイルに対して観測する。

// テストコードでは unwrap / expect を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use core_domain::orchestration::{AutonomyMode, WorkflowExecution};
use core_domain::workspace::SpaceName;
use core_interface_adapter::FakeClock;
use core_interface_adapter::orchestration::{
    SqliteEventStore, StorePath, WorkflowExecutionRepositoryImpl,
};
use core_use_case::orchestration::{
    EventStoreError, GlobalSeqNr, JournalReader, WorkflowExecutionRepository,
};
use rusqlite::Connection;
use tempfile::TempDir;

use support::{AT, advanced, genesis, intent_id};

/// 固定時刻 (2026-08-23T00:00:00Z の epoch ms)。
const NOW_MS: u64 = 1_787_443_200_000;

/// 一時ディレクトリ配下の 1 つのストアファイル。
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
        WorkflowExecutionRepositoryImpl::new(self.store())
    }

    fn store(&self) -> SqliteEventStore<FakeClock> {
        SqliteEventStore::open(self.path.clone(), FakeClock::new(NOW_MS)).expect("ストアは開ける")
    }
}

/// 5 コマンドぶん書き進め、最後の集約 (版を載せ替え済み) を返す。
async fn write_five(repository: &WorkflowExecutionRepositoryImpl<FakeClock>) -> WorkflowExecution {
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
    aggregate = advanced(aggregate, &event);

    let event = aggregate
        .approve_gate(Some("ok".to_string()), None, AT)
        .expect("承認");
    repository.store(&event, &aggregate).await.expect("4 件目");
    aggregate = advanced(aggregate, &event);

    let event = aggregate
        .set_autonomy(AutonomyMode::Autonomous, AT)
        .expect("自律モードの設定");
    repository.store(&event, &aggregate).await.expect("5 件目");
    advanced(aggregate, &event)
}

#[tokio::test]
async fn a_new_connection_after_a_crash_reconstructs_the_same_aggregate() {
    let fixture = Fixture::new();

    let expected = {
        let repository = fixture.repository();
        let expected = write_five(&repository).await;
        // ここで「プロセスが落ちる」— Repository も接続も drop される。
        expected
    };

    let reopened = fixture.repository();
    let found = reopened.find_by_id(&intent_id()).await.expect("読み直せる");
    assert_eq!(found.state(), expected.state(), "16 属性が一致する");
    assert_eq!(found.version(), 5);
    assert_eq!(found.seq_nr(), 5);
}

#[tokio::test]
async fn a_new_connection_after_a_crash_reads_the_whole_journal() {
    let fixture = Fixture::new();
    {
        let repository = fixture.repository();
        write_five(&repository).await;
    }

    let reader = fixture.store();
    let rows = reader.events_after(GlobalSeqNr::ZERO).await.expect("全件");
    assert_eq!(rows.len(), 5, "COMMIT 済みの 5 件が残る");
    assert_eq!(
        rows.iter()
            .map(|(global, _)| global.value())
            .collect::<Vec<_>>(),
        [1, 2, 3, 4, 5]
    );
    assert_eq!(
        rows.iter()
            .map(|(_, event)| event.seq_nr())
            .collect::<Vec<_>>(),
        [1, 2, 3, 4, 5]
    );
}

#[tokio::test]
async fn a_transaction_abandoned_by_a_crash_leaves_nothing_behind() {
    let fixture = Fixture::new();
    {
        let repository = fixture.repository();
        write_five(&repository).await;
    }

    // COMMIT を通らない Tx を開いたまま接続を捨てる (= Tx 途中のクラッシュ)。
    {
        let holder = Connection::open(fixture.path.as_path()).expect("生の接続");
        holder
            .execute_batch(
                "BEGIN IMMEDIATE;
                 INSERT INTO journal(aggregate_id, seq_nr, schema_version, event_type, payload, occurred_at)
                 VALUES ('01a02785-1bd8-76eb-aeea-5aa303ebd5b6', 6, 1, 'Unparked', '{\"type\":\"Unparked\"}', '2026-08-23T00:00:00Z');",
            )
            .expect("書きかけ");
    }

    let reader = fixture.store();
    let rows = reader.events_after(GlobalSeqNr::ZERO).await.expect("全件");
    assert_eq!(rows.len(), 5, "書きかけの 6 件目は残らない");

    let repository = fixture.repository();
    let found = repository.find_by_id(&intent_id()).await.expect("読める");
    assert_eq!(found.version(), 5);
}

#[tokio::test]
async fn the_store_survives_being_opened_and_closed_repeatedly() {
    let fixture = Fixture::new();
    {
        let repository = fixture.repository();
        write_five(&repository).await;
    }

    for _ in 0..3 {
        let repository = fixture.repository();
        let found = repository.find_by_id(&intent_id()).await.expect("読める");
        assert_eq!(found.version(), 5);
    }

    // 開き直しでスキーマを作り直したりしないこと (`user_version` は 1 のまま)。
    let conn = Connection::open(fixture.path.as_path()).expect("生の接続");
    let user_version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("user_version");
    assert_eq!(user_version, 1);
    let journal_rows: i64 = conn
        .query_row("SELECT count(*) FROM journal", [], |row| row.get(0))
        .expect("件数");
    assert_eq!(journal_rows, 5);
}

/// クラッシュ後に開いた別インスタンスからの書込は、続きの `seq_nr` から進む。
#[tokio::test]
async fn writing_resumes_from_the_persisted_version_after_a_crash() {
    let fixture = Fixture::new();
    {
        let repository = fixture.repository();
        write_five(&repository).await;
    }

    let repository = fixture.repository();
    let mut aggregate = repository.find_by_id(&intent_id()).await.expect("再水和");
    let event = aggregate
        .approve_gate(Some("ok".to_string()), None, AT)
        .or_else(|_| aggregate.complete_stage(AT))
        .expect("次のコマンド");
    assert_eq!(event.seq_nr(), 6);
    repository.store(&event, &aggregate).await.expect("6 件目");

    let reader = fixture.store();
    let rows: Result<Vec<_>, EventStoreError> = reader.events_after(GlobalSeqNr::new(5)).await;
    assert_eq!(rows.expect("差分").len(), 1);
}
