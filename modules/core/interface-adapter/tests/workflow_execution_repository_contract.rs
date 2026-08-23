//! `WorkflowExecutionRepository` / `JournalReader` の契約テスト — in-memory 実装 (BR2.7)。
//!
//! 契約そのものは `support/contract.rs` のジェネリック関数が持つ。本ファイルは in-memory
//! 実装をその関数群に流し込むだけである。SQLite 実装も同じ関数群を通す。

// テストコードでは unwrap / expect を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use core_interface_adapter::orchestration::{
    InMemoryEventStore, InMemoryWorkflowExecutionRepository,
};
use support::{StoreFixture, contract};

/// 1 つの in-memory ストアを共有し、そこへ何度でも Repository / Reader を開く試験装置。
struct InMemoryFixture {
    store: InMemoryEventStore,
}

impl InMemoryFixture {
    fn new() -> InMemoryFixture {
        InMemoryFixture {
            store: InMemoryEventStore::new(),
        }
    }
}

impl StoreFixture for InMemoryFixture {
    type Repository = InMemoryWorkflowExecutionRepository;
    type Reader = InMemoryEventStore;

    fn open(&self) -> InMemoryWorkflowExecutionRepository {
        InMemoryWorkflowExecutionRepository::with_store(self.store.clone())
    }

    fn reader(&self) -> InMemoryEventStore {
        self.store.clone()
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
    SqliteEventStore, StorePath, WorkflowExecutionRepositoryImpl,
};
use tempfile::TempDir;

/// 契約テストの `updated_at` を決める固定時刻 (2026-08-23T00:00:00Z の epoch ms)。
const NOW_MS: u64 = 1_787_443_200_000;

/// 1 つの SQLite ファイルを共有し、そこへ何度でも Repository / Reader を開く試験装置。
///
/// `open()` は同じファイルへの**新しい接続**を開く — in-memory 側のハンドル複製に対応する
/// 「別プロセスから同じストアを開き直す」形である。
struct SqliteFixture {
    /// 一時ディレクトリは fixture が生きているあいだ保持する (drop でファイルごと消える)。
    _dir: TempDir,
    path: StorePath,
}

impl SqliteFixture {
    fn new() -> SqliteFixture {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        let path = StorePath::for_space(&dir.path().join("aidlc"), &SpaceName::default_space());
        // `intents/` は upstream の既存ディレクトリ — ストアは作らない (BR2.1)。
        std::fs::create_dir_all(path.as_path().parent().expect("親 dir を持つ"))
            .expect("intents/ を先に作る");
        SqliteFixture { _dir: dir, path }
    }

    fn store(&self) -> SqliteEventStore<FakeClock> {
        SqliteEventStore::open(self.path.clone(), FakeClock::new(NOW_MS)).expect("ストアは開ける")
    }
}

impl StoreFixture for SqliteFixture {
    type Repository = WorkflowExecutionRepositoryImpl<FakeClock>;
    type Reader = SqliteEventStore<FakeClock>;

    fn open(&self) -> WorkflowExecutionRepositoryImpl<FakeClock> {
        WorkflowExecutionRepositoryImpl::new(self.store())
    }

    fn reader(&self) -> SqliteEventStore<FakeClock> {
        self.store()
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
