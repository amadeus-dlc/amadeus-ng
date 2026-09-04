//! ドメインイベント 16 変種の永続化 DTO — ジャーナル行 `payload` 列のバイト形 (**読む側**)。
//!
//! 外部タグ付き列挙 (`{"Started": { .. }}`)。**変種名・フィールド名・並びが契約**である。
//!
//! 全変種が `id` (イベント自身の識別子) と `aggregate_id` (どの集約の事実か) をこの順で
//! 先頭に持つ — ドメインイベントはエンティティの一種だからである (オーナー裁定 2026-09-02)。
//! `Unparked` はドメインの材料を持たないが識別子は運ぶので、単位変種ではなく構造体である。
//!
//! 各変種の材料 DTO は自分専用のファイルに 1 型 1 ファイルで置き (`one-public-type`)、
//! `of` / `to_domain` もそれぞれの型が持つ。ここでの `of` / `to_domain` は各変種への
//! 委譲だけを行う (`coding-rules/abstract-data-type.md` — 1 ファイル 1 公開型)。

use core_command_domain::orchestration::{
    IntentExecutionEvent, IntentExecutionEventId, IntentExecutionId,
};
use core_command_domain::workflow_definition::StageSlug;
use serde::{Deserialize, Serialize};

use super::autonomy_mode_set_dto::AutonomyModeSetDto;
use super::dto_decode_error::DtoDecodeError;
use super::gate_approved_dto::GateApprovedDto;
use super::gate_opened_dto::GateOpenedDto;
use super::gate_rejected_dto::GateRejectedDto;
use super::jumped_dto::JumpedDto;
use super::parked_dto::ParkedDto;
use super::practices_affirmed_dto::PracticesAffirmedDto;
use super::recomposed_dto::RecomposedDto;
use super::review_completed_dto::ReviewCompletedDto;
use super::review_requested_dto::ReviewRequestedDto;
use super::single_stage_run_committed_dto::SingleStageRunCommittedDto;
use super::skeleton_stance_recorded_dto::SkeletonStanceRecordedDto;
use super::stage_revised_dto::StageRevisedDto;
use super::stage_skipped_dto::StageSkippedDto;
use super::started_dto::StartedDto;
use super::unparked_dto::UnparkedDto;

/// ジャーナル行 `payload` の形。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentExecutionEventDto {
    /// 実行の開始 (解決済み計画を自己完結で持つ)。
    Started(StartedDto),
    /// 承認ゲートの開放。
    GateOpened(GateOpenedDto),
    /// 承認ゲートの通過。
    GateApproved(GateApprovedDto),
    /// 承認ゲートでの差し戻し。
    GateRejected(GateRejectedDto),
    /// 差し戻し後のゲート再入。
    StageRevised(StageRevisedDto),
    /// ステージの読み飛ばし。
    StageSkipped(StageSkippedDto),
    /// カーソルの移動。
    Jumped(JumpedDto),
    /// park マーカーの設置。
    Parked(ParkedDto),
    /// park マーカーの除去 (ドメインの材料は無いが識別子は運ぶ)。
    Unparked(UnparkedDto),
    /// 実効プランの再形成。
    Recomposed(RecomposedDto),
    /// 自律モードの設定。
    AutonomyModeSet(AutonomyModeSetDto),
    /// 隔離実行 (`--single`) の疑似ワークフロー ID 付き対の記録。
    SingleStageRunCommitted(SingleStageRunCommittedDto),
    /// walking-skeleton stance の記録。
    SkeletonStanceRecorded(SkeletonStanceRecordedDto),
    /// レビュアーの差し向け。
    ReviewRequested(ReviewRequestedDto),
    /// レビュアーの判定の記録。
    ReviewCompleted(ReviewCompletedDto),
    /// 承認された実践がメモリ層の正本へ書き写された事実。
    PracticesAffirmed(PracticesAffirmedDto),
}

/// イベント識別子の復号 (全変種が共有する private 補助 — 主たる従属先はこのファイル)。
pub(super) fn event_id_of(raw: &str) -> Result<IntentExecutionEventId, DtoDecodeError> {
    IntentExecutionEventId::parse(raw).map_err(|_| DtoDecodeError::malformed("id", raw))
}

/// 集約識別子 (どの実行の事実か) の復号。
pub(super) fn aggregate_id_of(raw: &str) -> Result<IntentExecutionId, DtoDecodeError> {
    IntentExecutionId::parse(raw).map_err(|_| DtoDecodeError::malformed("aggregate_id", raw))
}

/// ステージ参照の綴り。
///
/// `IntentExecutionEventDto` の変種 DTO 複数から共有される private 補助 —
/// 主たる従属先であるこのファイルに置き `pub(crate)` へ昇格する
/// (`coding-rules/module-visibility.md` の「複数公開型に共有される private 補助」)。
pub(crate) fn slug_spelling(slug: &StageSlug) -> String {
    slug.as_str().to_string()
}

/// ステージ参照の復号。
pub(crate) fn slug_of(raw: &str, field: &'static str) -> Result<StageSlug, DtoDecodeError> {
    StageSlug::parse(raw).map_err(|_| DtoDecodeError::malformed(field, raw))
}

/// ステージ参照の列の復号。
pub(crate) fn slugs_of(
    raw: &[String],
    field: &'static str,
) -> Result<Vec<StageSlug>, DtoDecodeError> {
    raw.iter().map(|value| slug_of(value, field)).collect()
}

impl IntentExecutionEventDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    #[must_use]
    pub fn of(event: &IntentExecutionEvent) -> IntentExecutionEventDto {
        match event {
            IntentExecutionEvent::Started(payload) => {
                IntentExecutionEventDto::Started(StartedDto::of(payload))
            }
            IntentExecutionEvent::GateOpened(payload) => {
                IntentExecutionEventDto::GateOpened(GateOpenedDto::of(payload))
            }
            IntentExecutionEvent::GateApproved(payload) => {
                IntentExecutionEventDto::GateApproved(GateApprovedDto::of(payload))
            }
            IntentExecutionEvent::GateRejected(payload) => {
                IntentExecutionEventDto::GateRejected(GateRejectedDto::of(payload))
            }
            IntentExecutionEvent::StageRevised(payload) => {
                IntentExecutionEventDto::StageRevised(StageRevisedDto::of(payload))
            }
            IntentExecutionEvent::StageSkipped(payload) => {
                IntentExecutionEventDto::StageSkipped(StageSkippedDto::of(payload))
            }
            IntentExecutionEvent::Jumped(payload) => {
                IntentExecutionEventDto::Jumped(JumpedDto::of(payload))
            }
            IntentExecutionEvent::Parked(payload) => {
                IntentExecutionEventDto::Parked(ParkedDto::of(payload))
            }
            IntentExecutionEvent::Unparked(payload) => {
                IntentExecutionEventDto::Unparked(UnparkedDto::of(payload))
            }
            IntentExecutionEvent::Recomposed(payload) => {
                IntentExecutionEventDto::Recomposed(RecomposedDto::of(payload))
            }
            IntentExecutionEvent::SingleStageRunCommitted(payload) => {
                IntentExecutionEventDto::SingleStageRunCommitted(SingleStageRunCommittedDto::of(
                    payload,
                ))
            }
            IntentExecutionEvent::ReviewRequested(payload) => {
                IntentExecutionEventDto::ReviewRequested(ReviewRequestedDto::of(payload))
            }
            IntentExecutionEvent::ReviewCompleted(payload) => {
                IntentExecutionEventDto::ReviewCompleted(ReviewCompletedDto::of(payload))
            }
            IntentExecutionEvent::PracticesAffirmed(payload) => {
                IntentExecutionEventDto::PracticesAffirmed(PracticesAffirmedDto::of(payload))
            }
            IntentExecutionEvent::SkeletonStanceRecorded(payload) => {
                IntentExecutionEventDto::SkeletonStanceRecorded(SkeletonStanceRecordedDto::of(
                    payload,
                ))
            }
            IntentExecutionEvent::AutonomyModeSet(payload) => {
                IntentExecutionEventDto::AutonomyModeSet(AutonomyModeSetDto::of(payload))
            }
        }
    }

    /// ドメインイベントへ戻す (読み)。
    ///
    /// # Errors
    ///
    /// 閉集合外の綴り・文法外のステージ参照・文法外の intent 識別子は `Malformed` を返す。
    pub fn to_domain(&self) -> Result<IntentExecutionEvent, DtoDecodeError> {
        Ok(match self {
            IntentExecutionEventDto::Started(payload) => {
                IntentExecutionEvent::Started(payload.to_domain()?)
            }
            IntentExecutionEventDto::GateOpened(payload) => {
                IntentExecutionEvent::GateOpened(payload.to_domain()?)
            }
            IntentExecutionEventDto::GateApproved(payload) => {
                IntentExecutionEvent::GateApproved(payload.to_domain()?)
            }
            IntentExecutionEventDto::GateRejected(payload) => {
                IntentExecutionEvent::GateRejected(payload.to_domain()?)
            }
            IntentExecutionEventDto::StageRevised(payload) => {
                IntentExecutionEvent::StageRevised(payload.to_domain()?)
            }
            IntentExecutionEventDto::StageSkipped(payload) => {
                IntentExecutionEvent::StageSkipped(payload.to_domain()?)
            }
            IntentExecutionEventDto::Jumped(payload) => {
                IntentExecutionEvent::Jumped(payload.to_domain()?)
            }
            IntentExecutionEventDto::Parked(payload) => {
                IntentExecutionEvent::Parked(payload.to_domain()?)
            }
            IntentExecutionEventDto::Unparked(payload) => {
                IntentExecutionEvent::Unparked(payload.to_domain()?)
            }
            IntentExecutionEventDto::Recomposed(payload) => {
                IntentExecutionEvent::Recomposed(payload.to_domain()?)
            }
            IntentExecutionEventDto::SingleStageRunCommitted(payload) => {
                IntentExecutionEvent::SingleStageRunCommitted(payload.to_domain()?)
            }
            IntentExecutionEventDto::ReviewRequested(payload) => {
                IntentExecutionEvent::ReviewRequested(payload.to_domain()?)
            }
            IntentExecutionEventDto::ReviewCompleted(payload) => {
                IntentExecutionEvent::ReviewCompleted(payload.to_domain()?)
            }
            IntentExecutionEventDto::PracticesAffirmed(payload) => {
                IntentExecutionEvent::PracticesAffirmed(payload.to_domain()?)
            }
            IntentExecutionEventDto::SkeletonStanceRecorded(payload) => {
                IntentExecutionEvent::SkeletonStanceRecorded(payload.to_domain()?)
            }
            IntentExecutionEventDto::AutonomyModeSet(payload) => {
                IntentExecutionEvent::AutonomyModeSet(payload.to_domain()?)
            }
        })
    }
}
