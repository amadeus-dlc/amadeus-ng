//! 読む側の永続化 DTO — ジャーナル行 `payload` 列を**この側の言葉で**読み戻す。
//!
//! ドメインは永続化知識から中立なので、行のバイトをどう読むかは読む側が自前で持つ
//! (`coding-rules/domain-persistence-neutrality.md`)。書く側 (command interface-adapter) の
//! DTO を**共有しない**のは意図的である — `coding-rules/cqrs-boundaries.md` の
//! 「共有部品は側の独立を DRY に優先する（側ごと専用化）」に従う。RMU の
//! [`CorruptCause`] がコマンド側と同名の別の型であるのと同じ理由で、ここも同名の別の型である。
//!
//! 書き手と読み手のワイヤ形式が一致していることは**横断適合テスト**が固定する
//! (`journal_protocol_conformance` / ゴールデンパリティ) — 型を共有して静的に揃えるのでは
//! なく、実際に書かれた行が実際に読めることで揃っていると示す。
//!
//! # ストア鍵の型はここに無い
//!
//! RMU の本番経路は `rusqlite` で `journal` 表を直接読むので、本家のイベントストアには
//! 触れない (`event-store-adapter-rs` は dev-dependency のままである)。本家の
//! `AggregateId` を満たす鍵が要るのは「本家が実際に書いた行」を用意するテストだけなので、
//! 鍵の型はテスト側に置く。
//!
//! # スナップショットは読まない
//!
//! RMU の仕事はジャーナルの横断読取と投影であり、スナップショット行には触れない
//! (`JournalReaderImpl` が読むのは `journal` 表と自前のチェックポイント表だけ)。したがって
//! この側にスナップショットの DTO は無い。
//!
//! [`CorruptCause`]: super::corrupt_cause::CorruptCause

mod autonomy_mode_set_dto;
mod defined_dto;
mod definition_content_dto;
mod dto_decode_error;
mod dto_vocabulary;
mod gate_approved_dto;
mod gate_opened_dto;
mod gate_rejected_dto;
mod intent_dto;
mod intent_event_dto;
mod intent_execution_event_dto;
mod jumped_dto;
mod kinds_codec;
mod parked_dto;
mod practices_affirmed_dto;
mod recomposed_dto;
mod redefined_dto;
mod review_completed_dto;
mod review_requested_dto;
mod single_stage_run_committed_dto;
mod skeleton_stance_recorded_dto;
mod stage_revised_dto;
mod stage_skipped_dto;
mod started_dto;
mod unparked_dto;
mod workflow_definition_event_dto;

pub use autonomy_mode_set_dto::AutonomyModeSetDto;
pub use dto_decode_error::DtoDecodeError;
pub use gate_approved_dto::GateApprovedDto;
pub use gate_opened_dto::GateOpenedDto;
pub use gate_rejected_dto::GateRejectedDto;
pub use intent_event_dto::IntentEventDto;
pub use intent_execution_event_dto::IntentExecutionEventDto;
pub use jumped_dto::JumpedDto;
pub use parked_dto::ParkedDto;
pub use practices_affirmed_dto::PracticesAffirmedDto;
pub use recomposed_dto::RecomposedDto;
pub use single_stage_run_committed_dto::SingleStageRunCommittedDto;
pub use skeleton_stance_recorded_dto::SkeletonStanceRecordedDto;
pub use stage_revised_dto::StageRevisedDto;
pub use stage_skipped_dto::StageSkippedDto;
pub use started_dto::StartedDto;
pub use workflow_definition_event_dto::WorkflowDefinitionEventDto;

#[cfg(test)]
mod definition_dto_tests;
#[cfg(test)]
mod tests;
