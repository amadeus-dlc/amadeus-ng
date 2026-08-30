//! `IntentRepository` の契約テスト (issue #50 — BR2.7 の形)。
//!
//! 契約そのものは `support/intent_contract.rs` のジェネリック関数が持つ。本ファイルは本家
//! event-store-adapter-rs の 2 バックエンド (memory / SQLite) と、結線テスト用の in-memory
//! ダブルを**同じ関数群**に流し込む。実 Repository の実装コードは 1 つしか無く、違うのは
//! 内包するストアだけである (ADR-010)。

// テストコードでは unwrap / expect を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::collections::HashMap;

use core_command_domain::workspace::{SpaceName, StorePath};
use core_command_interface_adapter::orchestration::{
    InMemoryIntentRepository, IntentMemoryStore, IntentRepositoryImpl, IntentSqliteStore,
};
use support::{IntentStoreFixture, intent_contract};
use tempfile::TempDir;

/// 揮発のストアを内包した Repository。
type MemoryRepository = IntentRepositoryImpl<IntentMemoryStore>;

/// SQLite ファイルのストアを内包した Repository。
type SqliteRepository = IntentRepositoryImpl<IntentSqliteStore>;

/// 本家 memory バックエンドの試験装置。
struct MemoryFixture;

impl IntentStoreFixture for MemoryFixture {
    type Repository = MemoryRepository;

    fn open(&self) -> MemoryRepository {
        IntentRepositoryImpl::in_memory()
    }

    fn reopen(&self, repository: &MemoryRepository) -> MemoryRepository {
        repository.reopened()
    }
}

/// 呼ぶたびに**別の SQLite ファイル**へストアを開く試験装置。
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

    /// まだ何も置かれていない場所を用意する (`intents/` は upstream の既存ディレクトリ —
    /// ストアは作らない)。
    fn fresh_path(&self) -> StorePath {
        let workspace = tempfile::Builder::new()
            .prefix("workspace-")
            .tempdir_in(self.root.path())
            .expect("open ごとの一時ディレクトリ")
            .keep();
        let path = StorePath::for_space(&workspace.join("aidlc"), &SpaceName::default());
        std::fs::create_dir_all(path.as_path().parent().expect("親 dir を持つ"))
            .expect("intents/ を先に作る");
        path
    }
}

impl IntentStoreFixture for SqliteFixture {
    type Repository = SqliteRepository;

    fn open(&self) -> SqliteRepository {
        IntentRepositoryImpl::open(&self.fresh_path()).expect("ストアは開ける")
    }

    fn reopen(&self, repository: &SqliteRepository) -> SqliteRepository {
        IntentRepositoryImpl::open(repository.path().expect("SQLite ストアは場所を持つ"))
            .expect("開き直せる")
    }
}

/// 結線テスト用 in-memory ダブルの試験装置 — 実物と**同じ契約**を通ることを固定する。
///
/// ダブルは「開き直し」を持たない (保持写像そのもの) ので、`reopen` は同じ保持を写した
/// 別インスタンスで代える。
struct DoubleFixture;

impl IntentStoreFixture for DoubleFixture {
    type Repository = InMemoryIntentRepository;

    fn open(&self) -> InMemoryIntentRepository {
        InMemoryIntentRepository::new(HashMap::new())
    }

    fn reopen(&self, repository: &InMemoryIntentRepository) -> InMemoryIntentRepository {
        repository.clone()
    }
}

macro_rules! contract_tests {
    ($($name:ident),* $(,)?) => {
        $(
            mod $name {
                use super::{DoubleFixture, MemoryFixture, SqliteFixture, intent_contract};

                #[tokio::test]
                async fn memory() {
                    intent_contract::$name(&MemoryFixture).await;
                }

                #[tokio::test]
                async fn sqlite() {
                    intent_contract::$name(&SqliteFixture::new()).await;
                }

                #[tokio::test]
                async fn wiring_double() {
                    intent_contract::$name(&DoubleFixture).await;
                }
            }
        )*
    };
}

contract_tests!(
    open_twice_yields_independent_empty_stores,
    round_trip,
    not_found,
    a_duplicate_genesis_is_a_conflict,
    a_mismatched_pair_is_refused,
);
