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
//! 12-workflow-definition が所有する。**利用者向けの upstream 逐語文言 (12 §4 / §6) は
//! ここには無い** — 定義 3 入力はリードモデルであり、それを読んで人に見せるのはクエリ側の
//! 仕事なので、文言の所有は `core-query-interface-adapter` へ移った
//! (`coding-rules/cqrs-boundaries.md` 規則 7、b26 段階 2)。ここに残るのは
//! `Error::source` の連鎖に載る開発者向け診断だけである。
//!
//! 読むだけの動詞 (`next` / `continue`) が要していた実装 — steering 連鎖の読取と継続トークンの
//! 封緘・開封 — も同じ Bolt でクエリ側へ移った (同規則 5)。
//!
//! 実装ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_command_interface_adapter::orchestration::<型>` で安定する
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。

mod intent_execution_repository_impl;
mod intent_repository_impl;
mod memory;
mod snapshot_strategy;
mod store_failure;
mod wire;
mod workflow_definition_repository_impl;

// 実 I/O Gateway (Repository 実装)
pub use intent_execution_repository_impl::IntentExecutionRepositoryImpl;
pub use intent_repository_impl::IntentRepositoryImpl;
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
