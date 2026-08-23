//! workspace コンテキストの実 Gateway (11-workspace §4)。ポート (trait) は core-use-case が
//! 所有し、ここでは実 I/O 実装 (と、必要になれば同階層のテスト用 in-memory 実装) を提供する。
//!
//! 状態ファイルの読取／書込は、集約 `WorkflowExecution` の Repository
//! (`WorkflowExecutionRepository`) が担う。その内部部品として `state_file_io` を先に置いてある
//! (ポートではない — aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md)。
//!
//! 時計は Gateway ではないのでここには無い。クレート root の `core_interface_adapter::Clock`
//! を参照。
//!
//! 実装ファイルの mod は private。公開 API は `pub use` が唯一の宣言であり、消費側のパスは
//! `core_interface_adapter::workspace::<型>` で安定する
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。現時点で
//! 公開する型は無い (`state_file_io` はクレート内部の部品)。

mod state_file_io;
