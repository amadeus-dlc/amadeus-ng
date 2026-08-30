//! orchestration コンテキストの**コマンド側**実 Gateway (10-orchestration §4)。ポート (trait)
//! は core-command-use-case が所有し、ここでは実 I/O 実装 (`...RepositoryImpl`) とテスト用
//! in-memory 実装 (`InMemory...`) を提供する
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md)。
//!
//! 集約の永続化そのものは本家 event-store-adapter-rs が担う (ADR-010)。ここに残るのは
//! 「本家に無いもの」— 集約の再構成手順を持つ Repository である。全集約横断の順序読取と
//! 投影チェックポイント (`JournalReaderImpl`) は RMU の仕事であり、2026-08-29 の側分割で
//! 中間クレート `core-read-model-updater` へ移った。
//!
//! `WorkflowDefinitionRepository` の規範 (3 入力の形状・読込失敗態度・述語 5 種) は
//! 12-workflow-definition が所有する。
//!
//! 実装ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_command_interface_adapter::orchestration::<型>` で安定する
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。

mod cli_wording;
mod command_spelling_impl;
mod continue_token_codec_impl;
mod intent_execution_repository_impl;
mod intent_repository_impl;
mod memory;
mod rule_bundle_source_impl;
mod snapshot_strategy;
mod store_failure;
mod wire;
mod workflow_definition_repository_impl;

// 実 I/O Gateway (Repository 実装)
pub use cli_wording::invalid_mode_message;
pub use command_spelling_impl::MulticallCommandSpelling;
pub use continue_token_codec_impl::ContinueTokenCodecImpl;
pub use intent_execution_repository_impl::IntentExecutionRepositoryImpl;
pub use intent_repository_impl::IntentRepositoryImpl;
pub use rule_bundle_source_impl::RuleBundleSourceImpl;
pub use snapshot_strategy::SnapshotStrategy;
pub use workflow_definition_repository_impl::WorkflowDefinitionRepositoryImpl;

// 永続化モデル (DTO) — ジャーナル行・スナップショット行のバイトを決めるのはこの層である
// (coding-rules/domain-persistence-neutrality.md)。ストアを具体化するのに型名が要るので
// 公開する — 見えるのはこのクレートの外の**アダプタ利用者**だけで、ドメインとユースケースは
// 依存の向き (層 = クレート) により参照できない。
pub use wire::{
    AggregateKey, IntentAggregateKey, WireAutonomyModeSet, WireDecodeError, WireEvent,
    WireGateApproved, WireGateOpened, WireGateRejected, WireIntent, WireIntentEvent,
    WireIntentExecution, WireJumped, WireParked, WireRecomposed, WireStageCompleted,
    WireStageRevised, WireStageSkipped, WireStarted,
};
// ストアの具体化 (バックエンドごとの別名 — 手順は同一)。
pub use intent_execution_repository_impl::{
    IntentExecutionMemoryStore, IntentExecutionSqliteStore,
};
pub use intent_repository_impl::{IntentMemoryStore, IntentSqliteStore};

// テスト用 in-memory 実装
pub use memory::{InMemoryIntentRepository, InMemoryWorkflowDefinitionRepository};

// 逐語文言の組み立て (12 §6 — レンダリングはアダプタ層に閉じる)
pub use workflow_definition_repository_impl::{
    graph_read_error_message, stage_graph_invalid_json_message, stage_graph_not_readable_message,
};
