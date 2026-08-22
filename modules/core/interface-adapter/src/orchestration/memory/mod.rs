//! テスト用 in-memory Gateway 実装 (12-workflow-definition §6 / 10-orchestration §8-3
//! 「テスト用 in-memory Gateway を最初に用意する」)。
//!
//! 本 mod 自体が private。公開は親 (`orchestration`) のファサードが再輸出する。

mod stage_graph_reader;

pub use stage_graph_reader::InMemoryStageGraphReader;
