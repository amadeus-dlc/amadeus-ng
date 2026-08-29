//! orchestration コンテキストの**コマンド側**ポート (trait) — 10-orchestration §3。実装
//! (Gateway) は `core-command-interface-adapter` に置く。ここには純粋なオーケストレーションと
//! trait 定義のみ (I/O 責務は持たない — 01 §7)。
//!
//! 読取側の語彙 (`JournalReader` / `JournalEntry` / `GlobalSeqNr` / `ProjectionName` /
//! `JournalReadError`) は 2026-08-29 の側分割で `core-read-model-updater` へ移った。
//! 呼ぶのは RMU だけなので、RMU クレート自身が所有する (ADR-009 2026-08-28 / 2026-08-29 追記)。
//!
//! 型ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_command_use_case::orchestration::<型>` で安定する
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。

mod corrupt_cause;
mod rehydrated_workflow_execution;
mod report_error;
mod report_outcome;
mod report_use_case;
mod reported_verdict;
mod repository_error;
#[cfg(test)]
mod test_support;
mod workflow_definition_repository;
mod workflow_execution_repository;

// ポート (trait) — Repository は集約名＋Repository で命名する
// (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md)。
// ES 形 Repository の動詞 store / find_by_id は本家ライブラリ由来の拡張語彙 (ADR-010)。
// 集約の永続化そのものは本家 event-store-adapter-rs が担うので、同形のローカル
// `EventStore` trait はもう置かない (ADR-010 — 借り物の契約を二重に書かない)。
pub use workflow_definition_repository::WorkflowDefinitionRepository;
pub use workflow_execution_repository::WorkflowExecutionRepository;

// ポートが返す読取レコード (本家の封筒型はポートから出さない — ADR-009 2026-08-28 追記)
pub use rehydrated_workflow_execution::RehydratedWorkflowExecution;

// ユースケース (CLI 動詞 = ユースケース)。入力は正規化済みの型で受け、出力は型で返す —
// 逐語文言は出す側 (合成ルートの Presenter) の持ち物である。
pub use report_use_case::ReportUseCase;
pub use reported_verdict::{ReportedTransition, ReportedVerdict};

// ユースケースの結果
pub use report_outcome::ReportOutcome;

// エラー
pub use corrupt_cause::CorruptCause;
pub use report_error::ReportError;
pub use repository_error::RepositoryError;
pub use workflow_definition_repository::GraphReadError;
