//! orchestration コンテキストのポート (trait) — 10-orchestration §3。実装 (Gateway) は
//! `core-interface-adapter` に置く。ここには純粋なオーケストレーションと trait 定義のみ
//! (I/O 責務は持たない — 01 §7)。
//!
//! 型ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_use_case::orchestration::<型>` で安定する
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。

mod event_store;
mod event_store_error;
mod global_seq_nr;
mod journal_reader;
mod projection_name;
mod repository_error;
mod workflow_definition_repository;
mod workflow_execution_repository;

// ポート (trait) — Repository は集約名＋Repository で命名する
// (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md)。
// ES 形 Repository の動詞 store / find_by_id は本家ライブラリ由来の拡張語彙 (ADR-006)。
pub use event_store::EventStore;
pub use journal_reader::JournalReader;
pub use workflow_definition_repository::WorkflowDefinitionRepository;
pub use workflow_execution_repository::WorkflowExecutionRepository;

// Domain Primitive (永続化の通番と投影の名前)
pub use global_seq_nr::GlobalSeqNr;
pub use projection_name::ProjectionName;

// エラー
pub use event_store_error::{CorruptCause, EventStoreError};
pub use projection_name::ProjectionNameError;
pub use repository_error::RepositoryError;
pub use workflow_definition_repository::GraphReadError;
