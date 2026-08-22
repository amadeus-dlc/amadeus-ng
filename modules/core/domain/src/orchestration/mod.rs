//! orchestration コンテキスト (10-orchestration.md) — 「次に何が起こるか」の Domain Primitive
//! と `WorkflowExecution` 集約。upstream 契約の逐語根拠は docs/specs/research/orchestration-*.md。
//!
//! 型ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_domain::orchestration::<型>` で安定する
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。

mod autonomy_mode;
mod jump_direction;
mod plan_action;
mod skeleton_stance;
mod verdict;
mod workflow_execution;

// Domain Primitive
pub use autonomy_mode::AutonomyMode;
pub use jump_direction::JumpDirection;
pub use plan_action::PlanAction;
pub use skeleton_stance::SkeletonStance;
pub use verdict::Verdict;

// 集約 (エンジンループの状態機械)
pub use workflow_execution::WorkflowExecution;

// 集約の観測結果
pub use workflow_execution::{EngineSignal, Status};

// 純関数ドメインサービス
pub use autonomy_mode::parse_mode_arg;

// エラー
pub use autonomy_mode::InvalidModeArg;
pub use skeleton_stance::UnknownStance;
pub use verdict::UnknownVerdict;
pub use workflow_execution::{CommandError, StartError};

// 逐語定数
pub use verdict::ACCEPTED_RESULTS;
