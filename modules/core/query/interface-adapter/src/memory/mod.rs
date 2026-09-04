//! テスト用 in-memory DAO 実装 — 実 I/O 無しでリードモデルの読取結果を差し替える口。
//!
//! テストダブルには `Impl` 接尾辞を付けない — `Impl` は「本物の Gateway 実装」の印である
//! (`coding-rules/gateway-taxonomy.md`)。読取専用ポートのダブルなので状態遷移も I/O も
//! 持たない。
//!
//! # 行を持ち、鍵で引く
//!
//! `read_*` 表を引く 13 本のダブルは **1 表ぶんの行を持ち、ポートが宣言した鍵で引く**。
//! 握った答えを鍵によらず返す形にしないのは、**同じ契約を両実装に課す**ためである
//! (`coding-rules/good-examples.md` §契約テスト) — 契約テストはジェネリック関数 1 本を
//! SQLite 実装とここのダブルに同一に走らせるので、鍵を見ないダブルでは「鍵が当たらなければ
//! `Ok(None)`」も「行が無いこと自体が答え」(`ScopeChangeDao` の無効 scope・
//! `SteeringPartDao` の終端) も表せない。
//!
//! 構築の作法は 3 本で揃えてある — `empty()` (行なし) から `with_row(..)` を重ね、
//! 引けない媒体は `failing(error)`。`with_row` が**明示的に取る鍵引数は View が運ばない
//! 列だけ**で、View が運ぶ鍵列 (実行の識別子・自然キー・束縛ダイジェスト) は行から読む —
//! 鍵と行の値がずれる余地を作らない。`failing` で組んだダブルに `with_row` を重ねても行は
//! 増えない (引けない媒体に読める行は無い)。
//!
//! use-case 層の単体テストはこれらを使えない (層 = クレートで依存が物理強制されており、
//! use-case はアダプタを知らない) — 向こう側のフェイクは `tests/` に置く。ここのダブルは
//! **アダプタ層とその上 (合成ルート)** のためのものである。
//!
//! 本 mod 自体が private。公開はクレート root のファサードが再輸出する。

mod in_memory_definition_dao;
mod in_memory_definition_stage_dao;
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

// 構造化リードモデル (`read_*` 表) を引く 13 ポートのダブル (b43 / b49)。
pub use in_memory_definition_dao::InMemoryDefinitionDao;
pub use in_memory_definition_stage_dao::InMemoryDefinitionStageDao;
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
