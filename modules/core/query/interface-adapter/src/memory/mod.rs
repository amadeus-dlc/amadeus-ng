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
mod in_memory_definition_dao;
mod in_memory_execution_dao;
mod in_memory_jump_dao;
mod in_memory_jump_phase_dao;
mod in_memory_next_answer_dao;
mod in_memory_phase_entry_dao;
mod in_memory_run_stage_dao;
mod in_memory_scope_change_dao;
mod in_memory_scope_dao;
mod in_memory_scope_keyword_dao;
mod in_memory_steering_part_dao;
mod in_memory_steering_plan_dao;
mod memory_rules_dao;
mod workflow_definition_dao;

pub use execution_state_dao::InMemoryExecutionStateDao;
pub use memory_rules_dao::InMemoryMemoryRulesDao;
pub use workflow_definition_dao::InMemoryWorkflowDefinitionDao;

// 構造化リードモデル (`read_*` 表) を引く 12 ポートのダブル (b43)。
pub use in_memory_definition_dao::InMemoryDefinitionDao;
pub use in_memory_execution_dao::InMemoryExecutionDao;
pub use in_memory_jump_dao::InMemoryJumpDao;
pub use in_memory_jump_phase_dao::InMemoryJumpPhaseDao;
pub use in_memory_next_answer_dao::InMemoryNextAnswerDao;
pub use in_memory_phase_entry_dao::InMemoryPhaseEntryDao;
pub use in_memory_run_stage_dao::InMemoryRunStageDao;
pub use in_memory_scope_change_dao::InMemoryScopeChangeDao;
pub use in_memory_scope_dao::InMemoryScopeDao;
pub use in_memory_scope_keyword_dao::InMemoryScopeKeywordDao;
pub use in_memory_steering_part_dao::InMemorySteeringPartDao;
pub use in_memory_steering_plan_dao::InMemorySteeringPlanDao;
