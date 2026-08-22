//! workspace コンテキストのポート (trait) — 11-workspace §3。実装 (Gateway) は
//! `core-interface-adapter` に置く。ここには純粋なオーケストレーションと trait 定義のみ
//! (I/O 責務は持たない — 01 §7)。
//!
//! 型ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_use_case::workspace::<型>` で安定する
//! (docs/memory/module-visibility.md)。

mod clock;
mod process_probe;
mod state_file_store;
mod workspace_lock;

// ポート (trait)
pub use clock::Clock;
pub use process_probe::ProcessProbe;
pub use state_file_store::StateFileStore;
pub use workspace_lock::WorkspaceLock;

// ポートの引数・戻り値 (取得予算 / 保持証明)
pub use workspace_lock::{AcquireBudget, LockGuard};

// エラー
pub use state_file_store::{StateFileReadError, StateFileWriteError};
pub use workspace_lock::AcquireError;
