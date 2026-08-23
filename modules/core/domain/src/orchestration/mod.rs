//! orchestration コンテキスト (10-orchestration.md) — 「次に何が起こるか」の Domain Primitive
//! と `WorkflowExecution` 集約。upstream 契約の逐語根拠は docs/specs/research/orchestration-*.md。
//!
//! # イベントソーシング形の集約 (ADR-001 / ADR-002)
//!
//! `WorkflowExecution` は **decide → 1 イベント → apply** で状態を進める。decide (12 コマンド) は
//! ガードを全て通してからイベントを 1 つ構築し、`apply_event` で自身に適用して返す。状態を動かす
//! のは `apply_event` だけなので、通常実行とリプレイは同一経路になる (BR1.1 / BR2.3)。
//! 永続化境界は `state()` / `from_state()` の値オブジェクトで、集約は serde を知らない。
//!
//! | コマンド | イベント |
//! |---|---|
//! | `start` / `start_with_entries` | `Started` (解決済み計画を自己完結で持つ) |
//! | `complete_stage` | `StageCompleted` |
//! | `open_gate` | `GateOpened` |
//! | `approve_gate` | `GateApproved` |
//! | `reject_gate` | `GateRejected` |
//! | `revise_stage` | `StageRevised` |
//! | `skip_stage` | `StageSkipped` |
//! | `jump` | `Jumped` |
//! | `park` / `unpark` | `Parked` / `Unparked` |
//! | `recompose` | `Recomposed` |
//! | `set_autonomy` | `AutonomyModeSet` |
//!
//! `next_decision` / `jump_resolve` / `stale_report` はクエリ (書込なし)。`EngineSignal` は
//! `NextDecision` から導出する 4 値である。
//!
//! # ゲート判定はフェーズで決まる (BR1.3)
//!
//! `gated(s) = stages[s].phase != initialization`。索引 0 の特別扱いはしない — 出荷グラフの
//! initialization は 3 ステージ (`workspace-scaffold` / `workspace-detection` / `state-init`) あり、
//! そのいずれも承認ゲートを持たない。非ゲートは `complete_stage`、ゲートは `approve_gate` で完了する。
//!
//! # Quint モデルとの射影 (BR2.5)
//!
//! | Quint (`engine_loop.qnt`) | 集約 |
//! |---|---|
//! | `status = Running` | `status = Running` ∧ `!parked_active()` |
//! | `status = WorkflowParked` | `parked_active()` (= `parked_at == Some(cursor)`) |
//! | `status = WorkflowCompleted` | `status = Completed` |
//! | `parkedAt = -1` | `parked_at = None` |
//! | `autonomous` | `autonomy = Autonomous` |
//! | `actSetAutonomy` (トグル) | `set_autonomy(反転値)` |
//! | `actRecompose` (1 ステージ) | `recompose(&[stage])` (要素数 1) |
//! | `lastDirective` | `EngineSignal::from(&NextDecision)` |
//! | stage 0 (非ゲート) | initialization 1 ステージだけを持つ合成計画の索引 0 |
//!
//! モデルの `gated(s) = s != 0` は最後の行の抽象である。ITF 準拠テスト
//! (`tests/engine_loop_conformance.rs`) はその合成計画で駆動し、実グラフの 3 ステージ側は集約の
//! ユニットテストが固定する。
//!
//! 型ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_domain::orchestration::<型>` で安定する
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。

mod apply_error;
mod autonomy_mode;
mod command_error;
mod intent_id;
mod jump_direction;
mod next_decision;
mod phase_boundary;
mod skeleton_stance;
mod stage_entry;
mod stage_index;
mod start_error;
mod start_request;
mod state_error;
mod status;
mod verdict;
mod workflow_execution;
mod workflow_execution_event;
mod workflow_execution_state;

// Domain Primitive
pub use autonomy_mode::AutonomyMode;
pub use intent_id::IntentId;
pub use jump_direction::JumpDirection;
pub use phase_boundary::PhaseBoundary;
pub use skeleton_stance::SkeletonStance;
pub use stage_entry::StageEntry;
pub use stage_index::StageIndex;
pub use start_request::StartRequest;
pub use verdict::Verdict;

// 集約 (エンジンループの状態機械)
pub use workflow_execution::WorkflowExecution;

// 集約の観測結果
pub use next_decision::{EngineSignal, NextDecision, NextRequest};
pub use status::Status;
pub use workflow_execution_state::WorkflowExecutionState;

// ドメインイベント (C5 の語彙 — 12 変種)
pub use workflow_execution_event::{
    AutonomyModeSet, GateApproved, GateOpened, GateRejected, Jumped, Parked, Recomposed,
    StageCompleted, StageRevised, StageSkipped, Started, WorkflowExecutionEvent,
    WorkflowExecutionEventPayload,
};

// ビルダー
pub use workflow_execution_state::WorkflowExecutionStateBuilder;

// エラー
pub use apply_error::ApplyError;
pub use autonomy_mode::InvalidModeArg;
pub use command_error::CommandError;
pub use intent_id::IntentIdError;
pub use skeleton_stance::UnknownStance;
pub use start_error::StartError;
pub use state_error::StateError;
pub use verdict::UnknownVerdict;

// 逐語定数
pub use verdict::ACCEPTED_RESULTS;
