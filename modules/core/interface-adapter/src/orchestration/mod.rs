//! orchestration コンテキストの実 Gateway (10-orchestration §4)。ポート (trait) は
//! core-use-case が所有し、ここでは実 I/O 実装 (`...RepositoryImpl`) とテスト用 in-memory
//! 実装 (`InMemory...`) を提供する (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md)。
//!
//! 集約の永続化そのものは本家 event-store-adapter-rs が担う (ADR-010)。ここに残るのは
//! 「本家に無いもの」— 集約の再構成手順を持つ Repository と、全集約横断の順序読取 +
//! 投影チェックポイント (`JournalReaderImpl`) である。
//!
//! `WorkflowDefinitionRepository` の規範 (3 入力の形状・読込失敗態度・述語 5 種) は
//! 12-workflow-definition が所有する。
//!
//! 実装ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_interface_adapter::orchestration::<型>` で安定する
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。

mod journal_reader_impl;
mod memory;
mod store_failure;
mod workflow_definition_repository_impl;
mod workflow_execution_repository_impl;

// 実 I/O Gateway (Repository 実装)
pub use workflow_definition_repository_impl::WorkflowDefinitionRepositoryImpl;
pub use workflow_execution_repository_impl::WorkflowExecutionRepositoryImpl;

// 投影が使う横断読取とチェックポイント (ストアファイルの場所 `StorePath` は共有語彙として
// `core_domain::workspace` が所有する)
pub use journal_reader_impl::JournalReaderImpl;

// テスト用 in-memory 実装
pub use memory::InMemoryWorkflowDefinitionRepository;

// 逐語文言の組み立て (12 §6 — レンダリングはアダプタ層に閉じる)
pub use workflow_definition_repository_impl::{
    graph_read_error_message, stage_graph_invalid_json_message, stage_graph_not_readable_message,
};
