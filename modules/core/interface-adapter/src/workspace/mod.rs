//! workspace コンテキストの実 Gateway (11-workspace §4)。ポート (trait) は core-use-case が
//! 所有し、ここでは実 I/O 実装とテスト用 in-memory 実装を提供する。

pub mod clock;
pub mod fs_state_file_store;
pub mod fs_workspace_lock;
pub mod memory;
pub mod process_probe;
pub mod testing;
