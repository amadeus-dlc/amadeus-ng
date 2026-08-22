//! orchestration コンテキストの実 Gateway (10-orchestration §4)。ポート (trait) は
//! core-use-case が所有し、ここでは実 I/O 実装 (`...RepositoryImpl`) とテスト用 in-memory
//! 実装 (`InMemory...`) を提供する (docs/memory/gateway-taxonomy.md)。
//!
//! `WorkflowDefinitionRepository` の規範 (3 入力の形状・読込失敗態度・述語 5 種) は
//! 12-workflow-definition が所有する。
//!
//! 実装ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_interface_adapter::orchestration::<型>` で安定する
//! (docs/memory/module-visibility.md)。

mod memory;
mod workflow_definition_repository_impl;

// 実 I/O Gateway (Repository 実装)
pub use workflow_definition_repository_impl::WorkflowDefinitionRepositoryImpl;

// テスト用 in-memory 実装
pub use memory::InMemoryWorkflowDefinitionRepository;

// 逐語文言の組み立て (12 §6 — レンダリングはアダプタ層に閉じる)
pub use workflow_definition_repository_impl::{
    read_error_message, stage_graph_invalid_json_message, stage_graph_not_readable_message,
};
