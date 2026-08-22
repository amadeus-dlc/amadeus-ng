//! workspace コンテキストの実 Gateway (11-workspace §4)。ポート (trait) は core-use-case が
//! 所有し、ここでは実 I/O 実装とテスト用 in-memory 実装を提供する。
//!
//! 実装ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_interface_adapter::workspace::<型>` で安定する
//! (docs/memory/module-visibility.md)。

mod clock;
mod fs_state_file_store;
mod fs_workspace_lock;
mod memory;
mod process_probe;
mod testing;

// 実 I/O Gateway
pub use fs_state_file_store::FsStateFileStore;
pub use fs_workspace_lock::FsWorkspaceLock;

// OS 由来の環境ポート実装
pub use clock::SystemClock;
pub use process_probe::OsProcessProbe;

// テスト用 in-memory / fake 実装
pub use memory::InMemoryStateFileStore;
pub use testing::{FakeClock, FakeProcessProbe};

// 既定しきい値 (upstream 逐語)
pub use fs_workspace_lock::{DEFAULT_LOCK_STALE_MS, DEFAULT_UNSTAMPED_GRACE_MS};
