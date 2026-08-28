//! クラッシュ再構成 (BR5.2 (a)) — 書込のあとプロセスが落ちたと見なし、新しい接続で
//! 同じ集約が同じ状態に戻ることを固定する。
//!
//! 「落ちた」は Repository (と本家ストアが握る接続) を drop することで表す。SQLite の
//! `COMMIT` を通った書込だけが残り、途中で捨てられた Tx は残らない — その 2 つを同じ
//! ファイルに対して観測する。

// テストコードでは unwrap / expect を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use core_domain::orchestration::{
    AutonomyMode, IntentId, WorkflowExecution, WorkflowExecutionEvent,
};
use core_domain::workspace::SpaceName;
use core_interface_adapter::orchestration::{
    JournalReaderImpl, StorePath, WorkflowExecutionRepositoryImpl,
};
use event_store_adapter_rs::EventStoreForSqlite;

use core_use_case::orchestration::{
    GlobalSeqNr, JournalEntry, JournalReadError, JournalReader, RehydratedWorkflowExecution,
    WorkflowExecutionRepository,
};
use rusqlite::Connection;
use tempfile::TempDir;

use support::{advance, at, intent_id, store_genesis};

/// Repository の具体型 (SQLite バックエンド)。
type Repository = WorkflowExecutionRepositoryImpl<
    EventStoreForSqlite<IntentId, WorkflowExecution, WorkflowExecutionEvent>,
>;

/// 一時ディレクトリ配下の 1 つのストアファイル。
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

    fn reader(&self) -> JournalReaderImpl {
        JournalReaderImpl::open(&self.path).expect("Reader は開ける")
    }
}

/// 5 コマンドぶん書き進め、最後の再水和結果を返す。
async fn write_five(repository: &mut Repository) -> RehydratedWorkflowExecution {
    let mut held = store_genesis(repository).await;
    held = advance(repository, &held, |aggregate| {
        aggregate.complete_stage(at())
    })
    .await;
    held = advance(repository, &held, |aggregate| {
        aggregate.open_gate(vec!["intent.md".to_string()], at())
    })
    .await;
    held = advance(repository, &held, |aggregate| {
        aggregate.approve_gate(Some("ok".to_string()), None, at())
    })
    .await;
    advance(repository, &held, |aggregate| {
        aggregate.switch_autonomy(AutonomyMode::Autonomous, at())
    })
    .await
}

#[tokio::test]
async fn a_new_connection_after_a_crash_reconstructs_the_same_aggregate() {
    let fixture = Fixture::new();

    let expected = {
        let mut repository = fixture.repository();
        let expected = write_five(&mut repository).await;
        // ここで「プロセスが落ちる」— Repository も接続も drop される。
        expected
    };

    let reopened = fixture.repository();
    let found = reopened.find_by_id(&intent_id()).await.expect("読み直せる");
    assert_eq!(
        found.aggregate().state(),
        expected.aggregate().state(),
        "16 属性が一致する"
    );
    assert_eq!(found.version(), 5);
    assert_eq!(found.aggregate().seq_nr(), 5);
}

#[tokio::test]
async fn a_new_connection_after_a_crash_reads_the_whole_journal() {
    let fixture = Fixture::new();
    {
        let mut repository = fixture.repository();
        write_five(&mut repository).await;
    }

    let reader = fixture.reader();
    let rows = reader.events_after(GlobalSeqNr::ZERO).await.expect("全件");
    assert_eq!(rows.len(), 5, "COMMIT 済みの 5 件が残る");
    assert_eq!(
        rows.iter()
            .map(|entry| entry.global_seq().to_u64())
            .collect::<Vec<_>>(),
        [1, 2, 3, 4, 5]
    );
    assert_eq!(
        rows.iter().map(JournalEntry::seq_nr).collect::<Vec<_>>(),
        [1, 2, 3, 4, 5]
    );
}

#[tokio::test]
async fn a_transaction_abandoned_by_a_crash_leaves_nothing_behind() {
    let fixture = Fixture::new();
    {
        let mut repository = fixture.repository();
        write_five(&mut repository).await;
    }

    // COMMIT を通らない Tx を開いたまま接続を捨てる (= Tx 途中のクラッシュ)。
    // 列は本家 `journal` のもの (スキーマガードテストがピン留めしている)。
    {
        let holder = Connection::open(fixture.path.as_path()).expect("生の接続");
        holder
            .execute_batch(
                "BEGIN IMMEDIATE;
                 INSERT INTO journal(pkey, skey, aid, seq_nr, payload, occurred_at, manifest)
                 VALUES ('p', 's-6', '01a02785-1bd8-76eb-aeea-5aa303ebd5b6', 6, X'7B7D', 0,
                         'workflow-execution-event/1');",
            )
            .expect("書きかけ");
    }

    let reader = fixture.reader();
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
        let mut repository = fixture.repository();
        write_five(&mut repository).await;
    }

    for _ in 0..3 {
        let repository = fixture.repository();
        let found = repository.find_by_id(&intent_id()).await.expect("読める");
        assert_eq!(found.version(), 5);
    }

    // 開き直しで表を作り直したりしないこと (本家の DDL は `IF NOT EXISTS`)。
    let conn = Connection::open(fixture.path.as_path()).expect("生の接続");
    let journal_rows: i64 = conn
        .query_row("SELECT count(*) FROM journal", [], |row| row.get(0))
        .expect("件数");
    assert_eq!(journal_rows, 5);
    let snapshot_rows: i64 = conn
        .query_row("SELECT count(*) FROM snapshot", [], |row| row.get(0))
        .expect("件数");
    assert_eq!(
        snapshot_rows, 1,
        "現行スロットの 1 行だけ (履歴は保持しない)"
    );
}

/// クラッシュ後に開いた別インスタンスからの書込は、続きの `seq_nr` から進む。
#[tokio::test]
async fn writing_resumes_from_the_persisted_version_after_a_crash() {
    let fixture = Fixture::new();
    {
        let mut repository = fixture.repository();
        write_five(&mut repository).await;
    }

    let mut repository = fixture.repository();
    let held = repository.find_by_id(&intent_id()).await.expect("再水和");
    let mut aggregate = held.aggregate().clone();
    let event = aggregate
        .approve_gate(Some("ok".to_string()), None, at())
        .or_else(|_| aggregate.complete_stage(at()))
        .expect("次のコマンド");
    assert_eq!(aggregate.seq_nr(), 6);
    repository
        .store(&event, &aggregate, held.version())
        .await
        .expect("6 件目");

    let reader = fixture.reader();
    let rows: Result<Vec<_>, JournalReadError> = reader.events_after(GlobalSeqNr::new(5)).await;
    assert_eq!(rows.expect("差分").len(), 1);
}
