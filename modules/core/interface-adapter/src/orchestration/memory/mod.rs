//! テスト用 in-memory Gateway 実装 (12-workflow-definition §6 / 10-orchestration §8-3
//! 「テスト用 in-memory Gateway を最初に用意する」)。
//!
//! テストダブルには `Impl` 接尾辞を付けない — `Impl` は「本物の Gateway 実装」の印である
//! (docs/memory/gateway-taxonomy.md)。
//!
//! 本 mod 自体が private。公開は親 (`orchestration`) のファサードが再輸出する。

mod workflow_definition_repository;

pub use workflow_definition_repository::InMemoryWorkflowDefinitionRepository;
