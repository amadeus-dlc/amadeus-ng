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
