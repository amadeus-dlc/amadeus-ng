//! orchestration コンテキストの実 Gateway (10-orchestration §4)。ポート (trait) は
//! core-use-case が所有し、ここでは実 I/O 実装とテスト用 in-memory 実装を提供する。
//!
//! `StageGraphReader` の規範 (3 入力の形状・読込失敗態度・述語 5 種) は
//! 12-workflow-definition が所有する。

pub mod fs_stage_graph_reader;
pub mod memory;
