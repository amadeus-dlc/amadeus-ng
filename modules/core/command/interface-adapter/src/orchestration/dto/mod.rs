//! 永続化モデル (DTO) — **ジャーナル行とスナップショット行のバイトを決めるのはここ**である。
//!
//! ドメインは永続化知識から中立であり、serde の記述もストアの trait 実装も持たない
//! (`coding-rules/domain-persistence-neutrality.md`)。直列化属性は「このフィールドはこの
//! 名前・この形でバイトになる」という**ストアとの契約**であって、ドメインの語彙ではない。
//!
//! # 往復の形
//!
//! - **書き**: ドメインの公開アクセサ → DTO → serde。
//! - **読み**: serde → DTO → ドメインの**検査付き再構成コンストラクタ**。
//!
//! Always Valid の担保は落ちない — 担保の場所がドメインの serde 属性からこの層の変換関数へ
//! 移るだけで、検査を迂回する構築口は存在しない (復元は必ず `From<Created>` /
//! `IntentExecution::new` を通る)。
//!
//! # 綴りの正本はここにある
//!
//! 閉集合の綴りは [`dto_vocabulary`] が持つ。ドメイン側の `as_str` / `parse` を**流用しない** —
//! 同じ値でも面ごとに綴りが違うからである (例: `PhaseId` はジャーナル上 `"Ideation"` だが
//! `stage-graph.json` 上は `"ideation"`、`BrownfieldGreenfield` はどちらも `"greenfield"`)。
//! 流用すると片方の綴りを変えた瞬間にもう片方のバイトが壊れる。
//!
//! 読む側 (RMU) は**自前の**復号 DTO を持つ (`coding-rules/cqrs-boundaries.md` — 共有部品は
//! 側の独立を DRY に優先する)。書き手と読み手のワイヤ形式の一致は横断適合テストが固定する。

mod autonomy_mode_set_dto;
mod created_dto;
mod defined_dto;
mod dto_decode_error;
mod dto_vocabulary;
mod gate_approved_dto;
mod gate_opened_dto;
mod gate_rejected_dto;
mod intent_aggregate_key_dto;
mod intent_dto;
mod intent_event_dto;
mod intent_execution_aggregate_key_dto;
mod intent_execution_dto;
mod intent_execution_event_dto;
mod jumped_dto;
mod parked_dto;
mod recomposed_dto;
mod redefined_dto;
mod stage_completed_dto;
mod stage_revised_dto;
mod stage_skipped_dto;
mod started_dto;
mod unparked_dto;
mod workflow_definition_aggregate_key_dto;
mod workflow_definition_dto;
mod workflow_definition_event_dto;

pub use autonomy_mode_set_dto::AutonomyModeSetDto;
pub use created_dto::CreatedDto;
pub use dto_decode_error::DtoDecodeError;
pub use gate_approved_dto::GateApprovedDto;
pub use gate_opened_dto::GateOpenedDto;
pub use gate_rejected_dto::GateRejectedDto;
pub use intent_aggregate_key_dto::IntentAggregateKeyDto;
pub use intent_dto::IntentDto;
pub use intent_event_dto::IntentEventDto;
pub use intent_execution_aggregate_key_dto::IntentExecutionAggregateKeyDto;
pub use intent_execution_dto::IntentExecutionDto;
pub use intent_execution_event_dto::IntentExecutionEventDto;
pub use jumped_dto::JumpedDto;
pub use parked_dto::ParkedDto;
pub use recomposed_dto::RecomposedDto;
pub use stage_completed_dto::StageCompletedDto;
pub use stage_revised_dto::StageRevisedDto;
pub use stage_skipped_dto::StageSkippedDto;
pub use started_dto::StartedDto;
pub use unparked_dto::UnparkedDto;
pub use workflow_definition_aggregate_key_dto::WorkflowDefinitionAggregateKeyDto;
pub use workflow_definition_dto::WorkflowDefinitionDto;
pub use workflow_definition_event_dto::WorkflowDefinitionEventDto;

#[cfg(test)]
mod tests;
