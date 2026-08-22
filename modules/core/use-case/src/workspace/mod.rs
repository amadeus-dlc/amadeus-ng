//! workspace コンテキストのポート (trait) — 11-workspace §3。実装は
//! `core-interface-adapter` に置く。ここには純粋なオーケストレーションと trait 定義のみ
//! (I/O 責務は持たない — 01 §7)。
//!
//! 本コンテキストのポートは並行性サービス `WorkspaceLock` だけである
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md):
//!
//! - 状態ファイルの読取／書込は集約 `WorkflowExecution` の Repository
//!   (`WorkflowExecutionRepository` — B-2 で設計) が担う。格納形式 (`aidlc-state.md`) は
//!   Repository 実装の内部詳細であり、ポート名にもポート表にも現れない。テスト用の
//!   `InMemoryWorkflowExecutionRepository` は B-2 で用意する。
//! - 時計・プロセス生存判定は**機構**であってアプリ境界のポートではない。実装と注入シームは
//!   アダプタ層 (`core_interface_adapter::{Clock, ProcessProbe}`) に置く。
//!
//! 型ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_use_case::workspace::<型>` で安定する
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。

mod workspace_lock;

// ポート (trait)
pub use workspace_lock::WorkspaceLock;

// ポートの引数・戻り値 (取得予算 / 保持証明)
pub use workspace_lock::{AcquireBudget, LockGuard};

// エラー
pub use workspace_lock::AcquireError;
