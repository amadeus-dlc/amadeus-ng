//! `WorkflowExecutionRepository` / `JournalReader` の契約テスト (BR2.7)。
//!
//! 契約そのものは `support/contract.rs` のジェネリック関数が持つ。本ファイルは in-memory /
//! SQLite の 2 実装を**同じ関数群**に流し込む。
//!
//! 末尾には、`StoreFixture` の doc が「契約の外」と明記した唯一の点 —— 開き直し / Reader を
//! 得た**後**の書込がそこから見えるか —— について、各実装が実際にどちらなのかを固定する
//! 実装固有テストを置く。契約の外であっても挙動が変われば必ず落ちる。

// テストコードでは unwrap / expect を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use core_interface_adapter::orchestration::{
    InMemoryEventStore, InMemoryWorkflowExecutionRepository,
};
use support::{StoreFixture, contract};

/// in-memory 実装の試験装置。
///
/// ストアは Repository の単一所有なので、開き直しと Reader は「書き終えた Repository が
/// 持つ 3 表を引き継いだ別インスタンス」で表す (`clone` は共有ハンドルではなく独立した
/// 写しである — coding-rules/interior-mutability.md)。`open()` が毎回まっさらな
/// `InMemoryEventStore` を作るので、2 度呼べば独立した 2 つの空のストアになる。
struct InMemoryFixture;

impl InMemoryFixture {
    const fn new() -> InMemoryFixture {
        InMemoryFixture
    }
}

impl StoreFixture for InMemoryFixture {
    type Repository = InMemoryWorkflowExecutionRepository;
    type Reader = InMemoryEventStore;

    fn open(&self) -> InMemoryWorkflowExecutionRepository {
        InMemoryWorkflowExecutionRepository::new()
    }

    fn reopen(
        &self,
        repository: &InMemoryWorkflowExecutionRepository,
    ) -> InMemoryWorkflowExecutionRepository {
        InMemoryWorkflowExecutionRepository::with_store(repository.event_store().clone())
    }

    fn reader(&self, repository: &InMemoryWorkflowExecutionRepository) -> InMemoryEventStore {
        repository.event_store().clone()
    }
}

macro_rules! contract_tests {
    ($($name:ident),* $(,)?) => {
        $(
            #[tokio::test]
            async fn $name() {
                contract::$name(&InMemoryFixture::new()).await;
            }
        )*
    };
}

contract_tests!(
    open_twice_yields_independent_empty_stores,
    reader_reflects_the_writes_completed_before_it_was_opened,
    reopen_reflects_the_writes_completed_before_it_was_reopened,
    round_trip,
    not_found,
    genesis_expects_version_zero,
    genesis_twice_conflicts,
    concurrent_rehydration_conflicts,
    sequence_gap_is_refused,
    mismatched_identity_is_refused,
    journal_reads_every_event_in_global_order,
    journal_reads_only_the_difference,
    unregistered_checkpoint_is_zero,
    checkpoint_advances_and_repeats_are_noops,
    checkpoint_regression_is_refused,
);

// ---------------------------------------------------------------------------
// SQLite 実装 — 同じ契約関数を通す (BR2.7: 片方だけ通るテストを残さない)
// ---------------------------------------------------------------------------

use core_domain::workspace::SpaceName;
use core_interface_adapter::FakeClock;
use core_interface_adapter::orchestration::{
    EventStoreImpl, StorePath, WorkflowExecutionRepositoryImpl,
};
use tempfile::TempDir;

/// 契約テストの `updated_at` を決める固定時刻 (2026-08-23T00:00:00Z の epoch ms)。
const NOW_MS: u64 = 1_787_443_200_000;

/// 呼ぶたびに**別の SQLite ファイル**へストアを開く試験装置。
///
/// `open()` が毎回新しいファイルを使うのは、[`StoreFixture::open`] の約束 (空のストアを指す
/// 新しい Repository) を SQLite でも真にするためである。以前は 1 つのファイルを抱え込んで
/// いたため、2 度目の `open()` が 1 度目の書込を見てしまい、in-memory 実装と意味が分岐して
/// いた。
///
/// 開き直し・Reader は、引数の Repository が持つ `EventStoreImpl::path()` からファイルを
/// 決める。どのストアを指すかは引数が決めるので、試験装置は単一のパスを持たない。
struct SqliteFixture {
    /// 一時ディレクトリは試験装置が生きているあいだ保持する (drop で配下ごと消える)。
    root: TempDir,
}

impl SqliteFixture {
    fn new() -> SqliteFixture {
        SqliteFixture {
            root: tempfile::tempdir().expect("一時ディレクトリ"),
        }
    }

    /// まだ何も置かれていない場所に新しいストアを開く。
    ///
    /// `keep()` で自動削除を外すのは、この場所を `root` の drop 一箇所で片付けるためである
    /// (Repository より先に消えると開き直しがファイルを見失う)。
    fn open_fresh(&self) -> EventStoreImpl<FakeClock> {
        let workspace = tempfile::Builder::new()
            .prefix("workspace-")
            .tempdir_in(self.root.path())
            .expect("open ごとの一時ディレクトリ")
            .keep();
        SqliteFixture::open_at(StorePath::for_space(
            &workspace.join("aidlc"),
            &SpaceName::default_space(),
        ))
    }

    /// 指定の場所へストアを開く (すでにあれば開き直し)。
    fn open_at(path: StorePath) -> EventStoreImpl<FakeClock> {
        // `intents/` は upstream の既存ディレクトリ — ストアは作らない (BR2.1)。
        std::fs::create_dir_all(path.as_path().parent().expect("親 dir を持つ"))
            .expect("intents/ を先に作る");
        EventStoreImpl::open(path, FakeClock::new(NOW_MS)).expect("ストアは開ける")
    }

    /// 引数の Repository が書いているファイルを開き直す。
    fn reopen_store(
        repository: &WorkflowExecutionRepositoryImpl<FakeClock>,
    ) -> EventStoreImpl<FakeClock> {
        SqliteFixture::open_at(repository.event_store().path().clone())
    }
}

impl StoreFixture for SqliteFixture {
    type Repository = WorkflowExecutionRepositoryImpl<FakeClock>;
    type Reader = EventStoreImpl<FakeClock>;

    fn open(&self) -> WorkflowExecutionRepositoryImpl<FakeClock> {
        WorkflowExecutionRepositoryImpl::new(self.open_fresh())
    }

    fn reopen(
        &self,
        repository: &WorkflowExecutionRepositoryImpl<FakeClock>,
    ) -> WorkflowExecutionRepositoryImpl<FakeClock> {
        // 同じファイルへの新しい接続 — 書き終えた行はファイルに残っている。
        WorkflowExecutionRepositoryImpl::new(SqliteFixture::reopen_store(repository))
    }

    fn reader(
        &self,
        repository: &WorkflowExecutionRepositoryImpl<FakeClock>,
    ) -> EventStoreImpl<FakeClock> {
        SqliteFixture::reopen_store(repository)
    }
}

macro_rules! sqlite_contract_tests {
    ($($name:ident => $contract:ident),* $(,)?) => {
        $(
            #[tokio::test]
            async fn $name() {
                contract::$contract(&SqliteFixture::new()).await;
            }
        )*
    };
}

sqlite_contract_tests!(
    sqlite_open_twice_yields_independent_empty_stores => open_twice_yields_independent_empty_stores,
    sqlite_reader_reflects_the_writes_completed_before_it_was_opened
        => reader_reflects_the_writes_completed_before_it_was_opened,
    sqlite_reopen_reflects_the_writes_completed_before_it_was_reopened
        => reopen_reflects_the_writes_completed_before_it_was_reopened,
    sqlite_round_trip => round_trip,
    sqlite_not_found => not_found,
    sqlite_genesis_expects_version_zero => genesis_expects_version_zero,
    sqlite_genesis_twice_conflicts => genesis_twice_conflicts,
    sqlite_concurrent_rehydration_conflicts => concurrent_rehydration_conflicts,
    sqlite_sequence_gap_is_refused => sequence_gap_is_refused,
    sqlite_mismatched_identity_is_refused => mismatched_identity_is_refused,
    sqlite_journal_reads_every_event_in_global_order => journal_reads_every_event_in_global_order,
    sqlite_journal_reads_only_the_difference => journal_reads_only_the_difference,
    sqlite_unregistered_checkpoint_is_zero => unregistered_checkpoint_is_zero,
    sqlite_checkpoint_advances_and_repeats_are_noops => checkpoint_advances_and_repeats_are_noops,
    sqlite_checkpoint_regression_is_refused => checkpoint_regression_is_refused,
);

// ---------------------------------------------------------------------------
// 契約の外 — 開き直し / Reader を得た「後」の書込の見え方 (実装ごとに固定する)
//
// `StoreFixture` の doc が逸脱として明記したとおり、ここは両実装で異なる。契約テストに
// 置けない代わりに、各実装が実際にどちらなのかをここで固定する。挙動が変われば落ちる。
// ---------------------------------------------------------------------------

use core_use_case::orchestration::{GlobalSeqNr, JournalReader, WorkflowExecutionRepository};
use support::{intent_id, store_genesis, store_stage_completed};

/// in-memory の Reader は 3 表の**写し**なので、開いた後の書込は見えない。
#[tokio::test]
async fn in_memory_reader_ignores_writes_made_after_it_was_opened() {
    let fixture = InMemoryFixture::new();
    let mut repository = fixture.open();
    let aggregate = store_genesis(&mut repository).await;

    let reader = fixture.reader(&repository);
    store_stage_completed(&mut repository, aggregate).await;

    let rows = reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("差分読取");
    assert_eq!(rows.len(), 1, "写しなので後の書込は見えない");
}

/// in-memory の開き直しも 3 表の**写し**なので、開き直した後の書込は見えない。
#[tokio::test]
async fn in_memory_reopened_repository_ignores_writes_made_after_it_was_reopened() {
    let fixture = InMemoryFixture::new();
    let mut repository = fixture.open();
    let aggregate = store_genesis(&mut repository).await;

    let reopened = fixture.reopen(&repository);
    store_stage_completed(&mut repository, aggregate).await;

    let found = reopened.find_by_id(&intent_id()).await.expect("読み直せる");
    assert_eq!(found.version(), 1, "写しなので後の書込は見えない");
}

/// SQLite の Reader は同じファイルへの**生きた接続**なので、開いた後の書込も見える。
#[tokio::test]
async fn sqlite_reader_observes_writes_made_after_it_was_opened() {
    let fixture = SqliteFixture::new();
    let mut repository = fixture.open();
    let aggregate = store_genesis(&mut repository).await;

    let reader = fixture.reader(&repository);
    store_stage_completed(&mut repository, aggregate).await;

    let rows = reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("差分読取");
    assert_eq!(rows.len(), 2, "生きた接続なので後の書込も見える");
}

/// SQLite の開き直しも**生きた接続**なので、開き直した後の書込も見える。
#[tokio::test]
async fn sqlite_reopened_repository_observes_writes_made_after_it_was_reopened() {
    let fixture = SqliteFixture::new();
    let mut repository = fixture.open();
    let aggregate = store_genesis(&mut repository).await;

    let reopened = fixture.reopen(&repository);
    store_stage_completed(&mut repository, aggregate).await;

    let found = reopened.find_by_id(&intent_id()).await.expect("読み直せる");
    assert_eq!(found.version(), 2, "生きた接続なので後の書込も見える");
}
