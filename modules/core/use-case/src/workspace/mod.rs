//! workspace コンテキストのポート (trait) — 11-workspace §3。実装 (Gateway) は
//! `core-interface-adapter` に置く。ここには純粋なオーケストレーションと trait 定義のみ
//! (I/O 責務は持たない — 01 §7)。

pub mod clock;
pub mod process_probe;
pub mod state_file_store;
pub mod workspace_lock;
