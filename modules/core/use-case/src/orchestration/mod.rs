//! orchestration コンテキストのポート (trait) — 10-orchestration §3。実装 (Gateway) は
//! `core-interface-adapter` に置く。ここには純粋なオーケストレーションと trait 定義のみ
//! (I/O 責務は持たない — 01 §7)。
//!
//! 型ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_use_case::orchestration::<型>` で安定する
//! (docs/memory/module-visibility.md)。

mod stage_graph_reader;

// ポート (trait)
pub use stage_graph_reader::StageGraphReader;

// エラー
pub use stage_graph_reader::GraphReadError;
