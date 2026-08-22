//! workspace コンテキストの実 Gateway (11-workspace §4)。ポート (trait) は core-use-case が
//! 所有し、ここでは実 I/O 実装 (と、必要になれば同階層のテスト用 in-memory 実装) を提供する。
//!
//! 状態ファイルの読取／書込は、集約 `WorkflowExecution` の Repository
//! (`WorkflowExecutionRepository` — B-2 で設計) が担う。その内部部品として `state_file_io` を
//! 先に置いてある (ポートではない — aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md)。テスト用の
//! `InMemoryWorkflowExecutionRepository` も B-2 で用意する。
//!
//! 時計・プロセス生存判定は Gateway ではないのでここには無い。クレート root の
//! `core_interface_adapter::{Clock, ProcessProbe}` を参照。
//!
//! 実装ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_interface_adapter::workspace::<型>` で安定する
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。

mod fs_workspace_lock;
mod state_file_io;

// 実 I/O Gateway
pub use fs_workspace_lock::FsWorkspaceLock;

// 既定しきい値 (upstream 逐語)
pub use fs_workspace_lock::{DEFAULT_LOCK_STALE_MS, DEFAULT_UNSTAMPED_GRACE_MS};
