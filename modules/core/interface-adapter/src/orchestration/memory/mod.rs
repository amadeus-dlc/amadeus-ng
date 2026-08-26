//! テスト用 in-memory Gateway 実装 (12-workflow-definition §6 / 10-orchestration §8-3
//! 「テスト用 in-memory Gateway を最初に用意する」)。
//!
//! テストダブルには `Impl` 接尾辞を付けない — `Impl` は「本物の Gateway 実装」の印である
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md)。
//!
//! 本 mod 自体が private。公開は親 (`orchestration`) のファサードが再輸出する。

mod in_memory_event_store;
mod workflow_definition_repository;
mod workflow_execution_repository;

pub use in_memory_event_store::InMemoryEventStore;
pub use workflow_definition_repository::InMemoryWorkflowDefinitionRepository;
pub use workflow_execution_repository::InMemoryWorkflowExecutionRepository;
