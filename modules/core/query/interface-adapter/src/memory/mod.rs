//! テスト用 in-memory DAO 実装 — 実 I/O 無しでリードモデルの読取結果を差し替える口。
//!
//! テストダブルには `Impl` 接尾辞を付けない — `Impl` は「本物の Gateway 実装」の印である
//! (`coding-rules/gateway-taxonomy.md`)。読取専用ポートのダブルなので握るのは
//! **1 回分の読取結果**だけで、状態遷移も I/O も持たない。
//!
//! use-case 層の単体テストはこれらを使えない (層 = クレートで依存が物理強制されており、
//! use-case はアダプタを知らない) — 向こう側のフェイクは
//! `core_query_use_case` の `orchestration::test_fixtures` にある。ここのダブルは
//! **アダプタ層とその上 (合成ルート)** のためのものである。
//!
//! 本 mod 自体が private。公開はクレート root のファサードが再輸出する。

mod execution_state_dao;
mod memory_rules_dao;
mod workflow_definition_dao;

pub use execution_state_dao::InMemoryExecutionStateDao;
pub use memory_rules_dao::InMemoryMemoryRulesDao;
pub use workflow_definition_dao::InMemoryWorkflowDefinitionDao;
