//! `WorkflowDefinitionRepository` の契約テスト (BR2.7 の形)。
//!
//! 契約そのものは `support/definition_contract.rs` のジェネリック関数が持つ。本ファイルは
//! 本家 event-store-adapter-rs の 2 バックエンド (memory / SQLite) を**同じ関数群**に
//! 流し込む。実 Repository の実装コードは 1 つしか無く、違うのは内包するストアだけである
//! (ADR-010)。
//!
//! **このファイルの存在自体が 2026-08-31 のオーナー裁定の検収である** — 定義の Repository が
//! イベントストア形になったからこそ、intent / intent-execution と同じ契約テストの形が
//! 書けるようになった。旧実装 (3 入力をファイルから読む) には「同じ約束を 2 つの
//! バックエンドに課す」という概念自体が存在しなかった。

// テストコードでは unwrap / expect を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use core_command_domain::workspace::{SpaceName, StorePath};
use core_command_interface_adapter::orchestration::{
    WorkflowDefinitionMemoryStore, WorkflowDefinitionRepositoryImpl, WorkflowDefinitionSqliteStore,
};
use support::{DefinitionStoreFixture, definition_contract};
use tempfile::TempDir;

/// 揮発のストアを内包した Repository。
type MemoryRepository = WorkflowDefinitionRepositoryImpl<WorkflowDefinitionMemoryStore>;

/// SQLite ファイルのストアを内包した Repository。
type SqliteRepository = WorkflowDefinitionRepositoryImpl<WorkflowDefinitionSqliteStore>;

/// 本家 memory バックエンドの試験装置。
struct MemoryFixture;

impl DefinitionStoreFixture for MemoryFixture {
    type Repository = MemoryRepository;

    fn open(&self) -> MemoryRepository {
        WorkflowDefinitionRepositoryImpl::in_memory()
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

impl DefinitionStoreFixture for SqliteFixture {
    type Repository = SqliteRepository;

    fn open(&self) -> SqliteRepository {
        WorkflowDefinitionRepositoryImpl::open(&self.fresh_path()).expect("ストアは開ける")
    }

    fn reopen(&self, repository: &SqliteRepository) -> SqliteRepository {
        WorkflowDefinitionRepositoryImpl::open(
            repository.path().expect("SQLite ストアは場所を持つ"),
        )
        .expect("開き直せる")
    }
}

macro_rules! contract_tests {
    ($($name:ident),* $(,)?) => {
        $(
            mod $name {
                use super::{MemoryFixture, SqliteFixture, definition_contract};

                #[tokio::test]
                async fn memory() {
                    definition_contract::$name(&MemoryFixture).await;
                }

                #[tokio::test]
                async fn sqlite() {
                    definition_contract::$name(&SqliteFixture::new()).await;
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
    a_redefinition_advances_the_stream,
    a_write_that_presents_a_stale_version_conflicts,
    an_event_from_another_definition_is_rejected_before_writing,
);
