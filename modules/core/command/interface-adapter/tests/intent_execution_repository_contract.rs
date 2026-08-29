//! `IntentExecutionRepository` の契約テスト (BR2.7)。
//!
//! 契約そのものは `support/contract.rs` のジェネリック関数が持つ。本ファイルは本家
//! event-store-adapter-rs の 2 バックエンド (memory / SQLite) を**同じ関数群**に流し込む。
//! Repository の実装コードは 1 つしか無く、違うのは内包するストアだけである (ADR-010)。

// テストコードでは unwrap / expect を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use core_command_domain::orchestration::{IntentExecution, IntentExecutionEvent, IntentId};
use core_command_domain::workspace::{SpaceName, StorePath};
use core_command_interface_adapter::orchestration::IntentExecutionRepositoryImpl;
use event_store_adapter_rs::{EventStoreForMemory, EventStoreForSqlite};
use support::{StoreFixture, contract};
use tempfile::TempDir;

/// 揮発のストアを内包した Repository。
type MemoryRepository = IntentExecutionRepositoryImpl<
    EventStoreForMemory<IntentId, IntentExecution, IntentExecutionEvent>,
>;

/// SQLite ファイルのストアを内包した Repository。
type SqliteRepository = IntentExecutionRepositoryImpl<
    EventStoreForSqlite<IntentId, IntentExecution, IntentExecutionEvent>,
>;

/// 本家 memory バックエンドの試験装置。
///
/// `open()` が毎回まっさらなストアを作るので、2 度呼べば独立した 2 つの空のストアになる。
/// 開き直しは `reopened()` — 本家の `Clone` は基底の表を共有するので、写しではなく
/// 同じストアを指す別の口になる。
struct MemoryFixture;

impl StoreFixture for MemoryFixture {
    type Repository = MemoryRepository;

    fn open(&self) -> MemoryRepository {
        IntentExecutionRepositoryImpl::in_memory()
    }

    fn reopen(&self, repository: &MemoryRepository) -> MemoryRepository {
        repository.reopened()
    }
}

/// 呼ぶたびに**別の SQLite ファイル**へストアを開く試験装置。
///
/// `open()` が毎回新しいファイルを使うのは、[`StoreFixture::open`] の約束 (空のストアを指す
/// 新しい Repository) を SQLite でも真にするためである。開き直しは同じファイルへの
/// **新しい接続** — 別プロセスからの再オープンと同じ形になる。
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

    /// まだ何も置かれていない場所を用意する。
    ///
    /// `keep()` で自動削除を外すのは、この場所を `root` の drop 一箇所で片付けるためである
    /// (Repository より先に消えると開き直しがファイルを見失う)。
    fn fresh_path(&self) -> StorePath {
        let workspace = tempfile::Builder::new()
            .prefix("workspace-")
            .tempdir_in(self.root.path())
            .expect("open ごとの一時ディレクトリ")
            .keep();
        let path = StorePath::for_space(&workspace.join("aidlc"), &SpaceName::default());
        // `intents/` は upstream の既存ディレクトリ — ストアは作らない (BR2.1)。
        std::fs::create_dir_all(path.as_path().parent().expect("親 dir を持つ"))
            .expect("intents/ を先に作る");
        path
    }
}

impl StoreFixture for SqliteFixture {
    type Repository = SqliteRepository;

    fn open(&self) -> SqliteRepository {
        IntentExecutionRepositoryImpl::open(&self.fresh_path()).expect("ストアは開ける")
    }

    fn reopen(&self, repository: &SqliteRepository) -> SqliteRepository {
        let path = repository.path().expect("SQLite なら場所を持つ");
        IntentExecutionRepositoryImpl::open(path).expect("同じファイルを開き直せる")
    }
}

macro_rules! contract_tests {
    ($($name:ident),* $(,)?) => {
        $(
            mod $name {
                use super::{MemoryFixture, SqliteFixture, contract};

                #[tokio::test]
                async fn memory() {
                    contract::$name(&MemoryFixture).await;
                }

                #[tokio::test]
                async fn sqlite() {
                    contract::$name(&SqliteFixture::new()).await;
                }
            }
        )*
    };
}

contract_tests!(
    open_twice_yields_independent_empty_stores,
    reopen_reflects_the_writes_completed_before_it_was_reopened,
    round_trip,
    not_found,
    the_store_assigns_the_first_version_on_genesis,
    genesis_twice_conflicts,
    concurrent_rehydration_conflicts,
    a_write_from_a_stale_version_conflicts,
    a_write_from_the_rehydrated_version_succeeds,
    a_genesis_with_a_non_zero_version_is_a_contract_violation,
);
