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

mod commit_error;
mod commit_verdict_use_case;
mod continue_use_case;
mod next_turn_input;
mod next_use_case;
mod port;
mod reported_transition;
#[cfg(test)]
mod test_support;
mod turn_materials;

// ポート (trait) — Repository は集約名＋Repository で命名する
// (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md)。
// ES 形 Repository の動詞 store / find_by_id は本家ライブラリ由来の拡張語彙 (ADR-010)。
// 集約の永続化そのものは本家 event-store-adapter-rs が担うので、同形のローカル
// `EventStore` trait はもう置かない (ADR-010 — 借り物の契約を二重に書かない)。
pub use port::{IntentExecutionRepository, IntentRepository};

// ユースケース。入力は正規化済みの型で受け、成功では何も返さない (CQS の Command —
// 「何が起きたか」は合成ルートが catch_up 後のリードモデルから導く)。逐語文言も出す側の
// 持ち物である。型名は upstream の CLI 動詞ではなく更新の意図から取る
// (オーナー裁定 2026-08-29 — 動詞 report は「レポート」と誤読される)。
pub use commit_verdict_use_case::CommitVerdictUseCase;
pub use continue_use_case::ContinueUseCase;
pub use next_turn_input::{ActiveWorkflow, NextTurnInput, NounFamily, NounToken, WorkspaceLayout};
pub use next_use_case::NextUseCase;

// 読取素材の値渡し (ポートではなく値 — issue #46。旧 `WorkflowDefinitionRepository` /
// `RuleBundleSource` ポートは廃止)。ユースケース層から見ると**ポートは Gateway のみ**で
// あり、現在の Gateway は Repository のみである (オーナー裁定 2026-08-30 —
// gateway-taxonomy の 2 責務のうち外部システムクライアントは現状存在しない。旧
// `ContinueTokenCodec` / `CommandSpelling` の廃止 (issue #45) と合わせてポート正常化完結)。
pub use reported_transition::ReportedTransition;
pub use turn_materials::{RuleUnreadable, TurnMaterials};

// エラー
pub use commit_error::CommitError;
pub use port::RepositoryError;
