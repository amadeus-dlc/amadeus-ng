//! orchestration コンテキストのポート (trait) — 10-orchestration §3。実装 (Gateway) は
//! `core-interface-adapter` に置く。ここには純粋なオーケストレーションと trait 定義のみ
//! (I/O 責務は持たない — 01 §7)。
//!
//! 型ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_use_case::orchestration::<型>` で安定する
//! (docs/memory/module-visibility.md)。

mod workflow_definition_repository;

// ポート (trait) — Repository は集約名＋Repository で命名する
// (docs/memory/gateway-taxonomy.md)。
pub use workflow_definition_repository::WorkflowDefinitionRepository;

// エラー
pub use workflow_definition_repository::GraphReadError;
