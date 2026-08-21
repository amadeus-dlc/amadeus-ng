//! orchestration コンテキストのポート (trait) — 10-orchestration §3。実装 (Gateway) は
//! `core-interface-adapter` に置く。ここには純粋なオーケストレーションと trait 定義のみ
//! (I/O 責務は持たない — 01 §7)。

pub mod stage_graph_reader;
