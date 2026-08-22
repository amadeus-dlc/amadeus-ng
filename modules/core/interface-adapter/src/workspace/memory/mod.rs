//! テスト用 in-memory Gateway 実装 (11-workspace §4「テスト用 in-memory 実装を最初に用意する」)。
//!
//! 本 mod 自体が private。公開は親 (`workspace`) のファサードが再輸出する。

mod state_file_store;

pub use state_file_store::InMemoryStateFileStore;
