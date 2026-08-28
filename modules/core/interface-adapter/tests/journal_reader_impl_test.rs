//! `JournalReaderImpl` の契約 (BR1.4) — 全集約横断の順序読取と投影チェックポイント。
//!
//! この面は本家 event-store-adapter-rs に無い (ADR-010 決定 4)。本家の `journal` 表を
//! 同じ DB ファイルへの**別接続**から `rowid` 順に読み、チェックポイントは我々の表に持つ。
//! したがって SQLite バックエンドにしか存在せず、Repository の契約テスト
//! (`workflow_execution_repository_contract.rs`) とは別ファイルに置く。
//!
//! 本家スキーマそのものへのピン留め (ガードテスト) は `journal_reader_impl.rs` の
//! インラインテストが持つ。ここが見るのは読み方の約束である。

// テストコードでは unwrap / expect を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

mod support;

use core_domain::workspace::SpaceName;
use core_interface_adapter::orchestration::{
    JournalReaderImpl, StorePath, WorkflowExecutionRepositoryImpl,
};
use core_use_case::orchestration::{
    CorruptCause, GlobalSeqNr, JournalReadError, JournalReader, ProjectionName,
    WorkflowExecutionRepository,
};
use event_store_adapter_rs::types::Event;
use rusqlite::Connection;
use tempfile::TempDir;

use support::{absent_intent_id, at, contract, genesis_for, intent_id, store_and_reload};

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

    fn repository(&self) -> impl WorkflowExecutionRepository + use<> {
        WorkflowExecutionRepositoryImpl::open(&self.path).expect("ストアは開ける")
    }

    fn reader(&self) -> JournalReaderImpl {
        JournalReaderImpl::open(&self.path).expect("Reader は開ける")
    }

    fn raw(&self) -> Connection {
        Connection::open(self.path.as_path()).expect("生の接続")
    }
}

fn projection() -> ProjectionName {
    ProjectionName::parse("state-file").expect("投影名は kebab")
}

// ---------------------------------------------------------------------------
// 差分読取 (BR1.4)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_journal_reads_every_event_in_global_order() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    contract::seed(&mut repository).await;

    let reader = fixture.reader();
    let rows = reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("差分読取");
    assert_eq!(rows.len(), 5);
    let globals: Vec<u64> = rows.iter().map(|(global, _)| global.to_u64()).collect();
    let mut sorted = globals.clone();
    sorted.sort_unstable();
    assert_eq!(globals, sorted, "global 通番の昇順");
    let seqs: Vec<usize> = rows.iter().map(|(_, event)| event.seq_nr()).collect();
    assert_eq!(seqs, [1, 2, 3, 4, 5], "欠落なく順に読める");
}

#[tokio::test]
async fn the_journal_reads_only_the_difference() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    contract::seed(&mut repository).await;

    let reader = fixture.reader();
    let all = reader.events_after(GlobalSeqNr::ZERO).await.expect("全件");
    let third = all.get(2).expect("3 件目").0;
    let rest = reader.events_after(third).await.expect("差分");
    assert_eq!(rest.len(), 2);
    assert!(rest.iter().all(|(global, _)| *global > third));
}

#[tokio::test]
async fn a_vacuum_rebuild_does_not_move_the_cursor() {
    // journal は削除ゼロの純追記 (DELETE は本家 v2.0.0 でも snapshot 表にしか無い) なので、
    // rowid は隙間の無い連番 1..N であり、VACUUM の再構築でも値が保たれる。ここはその前提を
    // 実挙動で釘留めする回帰テスト — 破れたら rowid カーソルの設計ごと見直すこと。
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    contract::seed(&mut repository).await;
    drop(repository);

    let mut reader = fixture.reader();
    let before = reader.events_after(GlobalSeqNr::ZERO).await.expect("全件");
    let third = before.get(2).expect("3 件目").0;
    reader
        .advance_checkpoint(&projection(), third)
        .await
        .expect("チェックポイント前進");
    drop(reader); // VACUUM は排他を要するため他接続を全部閉じてから実行する

    fixture
        .raw()
        .execute_batch("VACUUM")
        .expect("VACUUM は通る");

    let reader = fixture.reader();
    let after = reader.events_after(GlobalSeqNr::ZERO).await.expect("全件");
    let keys = |rows: &[(GlobalSeqNr, _)]| -> Vec<u64> {
        rows.iter()
            .map(|(global, _)| global.to_u64())
            .collect::<Vec<_>>()
    };
    assert_eq!(keys(&after), keys(&before), "rowid は VACUUM 前と同一");
    let saved = reader.checkpoint(&projection()).await.expect("保存済み");
    assert_eq!(saved, third, "チェックポイントも生きている");
    let resumed = reader.events_after(saved).await.expect("差分");
    assert_eq!(
        keys(&resumed),
        keys(&before)[3..].to_vec(),
        "続行が欠落も重複もしない"
    );
}

#[tokio::test]
async fn the_journal_interleaves_two_aggregates_in_commit_order() {
    // 本家のイベントストアは集約単位でしか読めない。横断のカーソルが「コミット順」で
    // あることこそ、この実装を自前で持つ理由である (ADR-010 決定 4)。
    let fixture = Fixture::new();
    let mut repository = fixture.repository();

    let (first, first_started) = genesis_for(intent_id());
    let first = store_and_reload(&mut repository, &first_started, &first).await;

    let (second, second_started) = genesis_for(absent_intent_id());
    store_and_reload(&mut repository, &second_started, &second).await;

    let mut first = first;
    let next = first.complete_stage(at()).expect("索引 0 は非ゲート");
    store_and_reload(&mut repository, &next, &first).await;

    let reader = fixture.reader();
    let rows = reader.events_after(GlobalSeqNr::ZERO).await.expect("全件");
    assert_eq!(
        rows.iter()
            .map(|(global, event)| (global.to_u64(), event.aggregate_id().as_str().to_string()))
            .collect::<Vec<_>>(),
        [
            (1, support::INTENT.to_string()),
            (2, support::ABSENT_INTENT.to_string()),
            (3, support::INTENT.to_string()),
        ],
        "書いた順に 1 本の列へ並ぶ"
    );
}

#[tokio::test]
async fn the_reader_observes_writes_made_after_it_was_opened() {
    // Reader は同じファイルへの**生きた接続**である (写しではない)。
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    let aggregate = support::store_genesis(&mut repository).await;

    let reader = fixture.reader();
    support::store_stage_completed(&mut repository, aggregate).await;

    let rows = reader.events_after(GlobalSeqNr::ZERO).await.expect("全件");
    assert_eq!(rows.len(), 2);
}

#[tokio::test]
async fn a_tampered_journal_payload_is_corrupt() {
    // 行を壊せるのは生の SQL だけである (実装に破壊用のフックを開けない — BR2.8)。
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    support::store_genesis(&mut repository).await;
    fixture
        .raw()
        .execute("UPDATE journal SET payload = X'7B6E6F74206A736F6E'", [])
        .expect("payload を壊す");

    let reader = fixture.reader();
    let err = reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect_err("復号できない");
    assert_eq!(
        err,
        JournalReadError::Corrupt {
            aggregate_id: support::INTENT.to_string(),
            seq_nr: None,
            cause: CorruptCause::UndecodablePayload,
        }
    );
}

// ---------------------------------------------------------------------------
// チェックポイント (BR1.4)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn an_unregistered_projection_reads_as_zero() {
    let fixture = Fixture::new();
    let _repository = fixture.repository();
    let reader = fixture.reader();
    assert_eq!(
        reader.checkpoint(&projection()).await.expect("読取"),
        GlobalSeqNr::ZERO
    );
}

#[tokio::test]
async fn the_checkpoint_advances_and_repeats_are_noops() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    contract::seed(&mut repository).await;

    let mut reader = fixture.reader();
    let rows = reader.events_after(GlobalSeqNr::ZERO).await.expect("全件");
    let last = rows.last().expect("5 件ある").0;

    reader
        .advance_checkpoint(&projection(), last)
        .await
        .expect("前進");
    assert_eq!(reader.checkpoint(&projection()).await.expect("読取"), last);

    reader
        .advance_checkpoint(&projection(), last)
        .await
        .expect("同値は no-op");
    assert_eq!(reader.checkpoint(&projection()).await.expect("読取"), last);
    assert!(
        reader.events_after(last).await.expect("差分").is_empty(),
        "追いついた投影に差分は無い"
    );
}

#[tokio::test]
async fn the_checkpoint_survives_reopening_the_store() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    contract::seed(&mut repository).await;
    {
        let mut reader = fixture.reader();
        reader
            .advance_checkpoint(&projection(), GlobalSeqNr::new(3))
            .await
            .expect("前進");
    }

    let reader = fixture.reader();
    assert_eq!(
        reader.checkpoint(&projection()).await.expect("読取"),
        GlobalSeqNr::new(3),
        "自前の表はファイルに残る"
    );
}

#[tokio::test]
async fn a_checkpoint_regression_is_refused() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    contract::seed(&mut repository).await;

    let mut reader = fixture.reader();
    reader
        .advance_checkpoint(&projection(), GlobalSeqNr::new(3))
        .await
        .expect("前進");
    let err = reader
        .advance_checkpoint(&projection(), GlobalSeqNr::new(2))
        .await
        .expect_err("後退は拒否");
    assert_eq!(
        err,
        JournalReadError::CheckpointRegression {
            projection: projection(),
            current: GlobalSeqNr::new(3),
            requested: GlobalSeqNr::new(2),
        }
    );
    assert_eq!(
        reader.checkpoint(&projection()).await.expect("読取"),
        GlobalSeqNr::new(3),
        "拒否しても現在値は動かない"
    );
}

#[tokio::test]
async fn two_projections_keep_independent_checkpoints() {
    let fixture = Fixture::new();
    let mut repository = fixture.repository();
    contract::seed(&mut repository).await;

    let other = ProjectionName::parse("intents-registry").expect("投影名は kebab");
    let mut reader = fixture.reader();
    reader
        .advance_checkpoint(&projection(), GlobalSeqNr::new(4))
        .await
        .expect("前進");
    assert_eq!(
        reader.checkpoint(&other).await.expect("読取"),
        GlobalSeqNr::ZERO,
        "別の投影は動かない"
    );
    reader
        .advance_checkpoint(&other, GlobalSeqNr::new(2))
        .await
        .expect("前進");
    assert_eq!(
        reader.checkpoint(&projection()).await.expect("読取"),
        GlobalSeqNr::new(4)
    );
    assert_eq!(
        reader.checkpoint(&other).await.expect("読取"),
        GlobalSeqNr::new(2)
    );
}
