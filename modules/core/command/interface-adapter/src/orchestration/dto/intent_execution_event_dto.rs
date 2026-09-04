//! ドメインイベント 16 変種の永続化 DTO — ジャーナル行 `payload` 列のバイト形。
//!
//! 外部タグ付き列挙 (`{"Started": { .. }}`)。**変種名・フィールド名・並びが契約**である。
//!
//! 全変種が `id` (イベント自身の識別子) と `aggregate_id` (どの集約の事実か) をこの順で
//! 先頭に持つ — ドメインイベントはエンティティの一種だからである (オーナー裁定 2026-09-02)。
//! `Unparked` はドメインの材料を持たないが識別子は運ぶので、単位変種ではなく構造体である。

use core_command_domain::orchestration::{
    AutonomyModeSet, GateApproved, GateOpened, GateRejected, IntentExecutionEvent,
    IntentExecutionEventId, IntentExecutionId, IntentId, Jumped, Parked, PracticesAffirmed,
    Recomposed, ReviewCompleted, ReviewRequested, SingleStageRunCommitted, SkeletonStanceRecorded,
    StageEntry, StageRevised, StageSkipped, Started, Unparked,
};
use core_command_domain::workflow_definition::StageSlug;
use serde::{Deserialize, Serialize};

use super::autonomy_mode_set_dto::AutonomyModeSetDto;
use super::dto_decode_error::DtoDecodeError;
use super::dto_vocabulary::{
    autonomy_of, autonomy_spelling, review_verdict_of, review_verdict_spelling, skeleton_stance_of,
    skeleton_stance_spelling,
};
use super::gate_approved_dto::GateApprovedDto;
use super::gate_opened_dto::GateOpenedDto;
use super::gate_rejected_dto::GateRejectedDto;
use super::intent_dto::StageEntryDto;
use super::jumped_dto::JumpedDto;
use super::parked_dto::ParkedDto;
use super::practices_affirmed_dto::{PracticesAffirmedDto, PromotedSectionDto};
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
    /// 実行の開始 (事実の主体 = intent の識別子だけ — issue #56)。
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

/// イベント識別子の復号。
fn event_id_of(raw: &str) -> Result<IntentExecutionEventId, DtoDecodeError> {
    IntentExecutionEventId::parse(raw).map_err(|_| DtoDecodeError::malformed("id", raw))
}

/// 集約識別子 (どの実行の事実か) の復号。
fn aggregate_id_of(raw: &str) -> Result<IntentExecutionId, DtoDecodeError> {
    IntentExecutionId::parse(raw).map_err(|_| DtoDecodeError::malformed("aggregate_id", raw))
}

/// ステージ参照の綴り。
fn slug_spelling(slug: &StageSlug) -> String {
    slug.as_str().to_string()
}

/// ステージ参照の復号。
fn slug_of(raw: &str, field: &'static str) -> Result<StageSlug, DtoDecodeError> {
    StageSlug::parse(raw).map_err(|_| DtoDecodeError::malformed(field, raw))
}

/// ステージ参照の列の復号。
fn slugs_of(raw: &[String], field: &'static str) -> Result<Vec<StageSlug>, DtoDecodeError> {
    raw.iter().map(|value| slug_of(value, field)).collect()
}

impl IntentExecutionEventDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    #[must_use]
    pub fn of(event: &IntentExecutionEvent) -> IntentExecutionEventDto {
        match event {
            IntentExecutionEvent::Started(payload) => {
                IntentExecutionEventDto::Started(StartedDto {
                    id: payload.id().as_str().to_string(),
                    aggregate_id: payload.aggregate_id().as_str().to_string(),
                    intent_id: payload.intent_id().as_str().to_string(),
                    stages: payload.stages().iter().map(StageEntryDto::of).collect(),
                })
            }
            IntentExecutionEvent::GateOpened(payload) => {
                IntentExecutionEventDto::GateOpened(GateOpenedDto {
                    id: payload.id().as_str().to_string(),
                    aggregate_id: payload.aggregate_id().as_str().to_string(),
                    stage: slug_spelling(payload.stage()),
                    artifacts: payload.artifacts().to_vec(),
                })
            }
            IntentExecutionEvent::GateApproved(payload) => {
                IntentExecutionEventDto::GateApproved(GateApprovedDto {
                    id: payload.id().as_str().to_string(),
                    aggregate_id: payload.aggregate_id().as_str().to_string(),
                    stage: slug_spelling(payload.stage()),
                    user_input: payload.user_input().map(str::to_string),
                })
            }
            IntentExecutionEvent::GateRejected(payload) => {
                IntentExecutionEventDto::GateRejected(GateRejectedDto {
                    id: payload.id().as_str().to_string(),
                    aggregate_id: payload.aggregate_id().as_str().to_string(),
                    stage: slug_spelling(payload.stage()),
                    feedback: payload.feedback().map(str::to_string),
                })
            }
            IntentExecutionEvent::StageRevised(payload) => {
                IntentExecutionEventDto::StageRevised(StageRevisedDto {
                    id: payload.id().as_str().to_string(),
                    aggregate_id: payload.aggregate_id().as_str().to_string(),
                    stage: slug_spelling(payload.stage()),
                })
            }
            IntentExecutionEvent::StageSkipped(payload) => {
                IntentExecutionEventDto::StageSkipped(StageSkippedDto {
                    id: payload.id().as_str().to_string(),
                    aggregate_id: payload.aggregate_id().as_str().to_string(),
                    stage: slug_spelling(payload.stage()),
                    reason: payload.reason().to_string(),
                })
            }
            IntentExecutionEvent::Jumped(payload) => IntentExecutionEventDto::Jumped(JumpedDto {
                id: payload.id().as_str().to_string(),
                aggregate_id: payload.aggregate_id().as_str().to_string(),
                target: slug_spelling(payload.target()),
            }),
            IntentExecutionEvent::Parked(payload) => IntentExecutionEventDto::Parked(ParkedDto {
                id: payload.id().as_str().to_string(),
                aggregate_id: payload.aggregate_id().as_str().to_string(),
                stage: slug_spelling(payload.stage()),
            }),
            IntentExecutionEvent::Unparked(payload) => {
                IntentExecutionEventDto::Unparked(UnparkedDto {
                    id: payload.id().as_str().to_string(),
                    aggregate_id: payload.aggregate_id().as_str().to_string(),
                })
            }
            IntentExecutionEvent::Recomposed(payload) => {
                IntentExecutionEventDto::Recomposed(RecomposedDto {
                    id: payload.id().as_str().to_string(),
                    aggregate_id: payload.aggregate_id().as_str().to_string(),
                    skipped: payload.skipped().iter().map(slug_spelling).collect(),
                    added: payload.added().iter().map(slug_spelling).collect(),
                })
            }
            IntentExecutionEvent::AutonomyModeSet(payload) => {
                IntentExecutionEventDto::AutonomyModeSet(AutonomyModeSetDto {
                    id: payload.id().as_str().to_string(),
                    aggregate_id: payload.aggregate_id().as_str().to_string(),
                    mode: autonomy_spelling(payload.mode()).to_string(),
                })
            }
            IntentExecutionEvent::SingleStageRunCommitted(payload) => {
                IntentExecutionEventDto::SingleStageRunCommitted(SingleStageRunCommittedDto {
                    id: payload.id().as_str().to_string(),
                    aggregate_id: payload.aggregate_id().as_str().to_string(),
                    stage: slug_spelling(payload.stage()),
                })
            }
            IntentExecutionEvent::SkeletonStanceRecorded(payload) => {
                IntentExecutionEventDto::SkeletonStanceRecorded(SkeletonStanceRecordedDto {
                    id: payload.id().as_str().to_string(),
                    aggregate_id: payload.aggregate_id().as_str().to_string(),
                    stance: skeleton_stance_spelling(payload.stance()).to_string(),
                })
            }
            IntentExecutionEvent::ReviewRequested(payload) => {
                IntentExecutionEventDto::ReviewRequested(ReviewRequestedDto {
                    id: payload.id().as_str().to_string(),
                    aggregate_id: payload.aggregate_id().as_str().to_string(),
                    stage: slug_spelling(payload.stage()),
                    reviewer: payload.reviewer().to_string(),
                    iteration: payload.iteration(),
                    retry: payload.is_retry(),
                })
            }
            IntentExecutionEvent::ReviewCompleted(payload) => {
                IntentExecutionEventDto::ReviewCompleted(ReviewCompletedDto {
                    id: payload.id().as_str().to_string(),
                    aggregate_id: payload.aggregate_id().as_str().to_string(),
                    stage: slug_spelling(payload.stage()),
                    reviewer: payload.reviewer().to_string(),
                    iteration: payload.iteration(),
                    verdict: review_verdict_spelling(payload.verdict()).to_string(),
                })
            }
            IntentExecutionEvent::PracticesAffirmed(payload) => {
                IntentExecutionEventDto::PracticesAffirmed(PracticesAffirmedDto {
                    id: payload.id().as_str().to_string(),
                    aggregate_id: payload.aggregate_id().as_str().to_string(),
                    stage: slug_spelling(payload.stage()),
                    affirming_user: payload.affirming_user().to_string(),
                    sections: payload
                        .sections()
                        .iter()
                        .map(PromotedSectionDto::of)
                        .collect(),
                    mandated: payload.mandated().to_vec(),
                    forbidden: payload.forbidden().to_vec(),
                })
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
                let stages = payload
                    .stages
                    .iter()
                    .map(StageEntryDto::to_domain)
                    .collect::<Result<Vec<StageEntry>, DtoDecodeError>>()?;
                // 計画そのものの不変条件はドメインが持つ (`StageEntry::check_plan`) —
                // 判断を DTO に複製せず呼ぶだけにする。ここで止めないと、破れた計画が
                // 集約の再構成まで届いてクラッシュする (再構成は失敗を返さない)。
                StageEntry::check_plan(&stages).map_err(|_| DtoDecodeError::InvariantViolation)?;
                IntentExecutionEvent::Started(Started::new(
                    event_id_of(&payload.id)?,
                    aggregate_id_of(&payload.aggregate_id)?,
                    IntentId::parse(&payload.intent_id)
                        .map_err(|_| DtoDecodeError::malformed("intent_id", &payload.intent_id))?,
                    stages,
                ))
            }
            IntentExecutionEventDto::GateOpened(payload) => {
                IntentExecutionEvent::GateOpened(GateOpened::new(
                    event_id_of(&payload.id)?,
                    aggregate_id_of(&payload.aggregate_id)?,
                    slug_of(&payload.stage, "stage")?,
                    payload.artifacts.clone(),
                ))
            }
            IntentExecutionEventDto::GateApproved(payload) => {
                IntentExecutionEvent::GateApproved(GateApproved::new(
                    event_id_of(&payload.id)?,
                    aggregate_id_of(&payload.aggregate_id)?,
                    slug_of(&payload.stage, "stage")?,
                    payload.user_input.clone(),
                ))
            }
            IntentExecutionEventDto::GateRejected(payload) => {
                IntentExecutionEvent::GateRejected(GateRejected::new(
                    event_id_of(&payload.id)?,
                    aggregate_id_of(&payload.aggregate_id)?,
                    slug_of(&payload.stage, "stage")?,
                    payload.feedback.clone(),
                ))
            }
            IntentExecutionEventDto::StageRevised(payload) => {
                IntentExecutionEvent::StageRevised(StageRevised::new(
                    event_id_of(&payload.id)?,
                    aggregate_id_of(&payload.aggregate_id)?,
                    slug_of(&payload.stage, "stage")?,
                ))
            }
            IntentExecutionEventDto::StageSkipped(payload) => {
                IntentExecutionEvent::StageSkipped(StageSkipped::new(
                    event_id_of(&payload.id)?,
                    aggregate_id_of(&payload.aggregate_id)?,
                    slug_of(&payload.stage, "stage")?,
                    payload.reason.clone(),
                ))
            }
            IntentExecutionEventDto::Jumped(payload) => IntentExecutionEvent::Jumped(Jumped::new(
                event_id_of(&payload.id)?,
                aggregate_id_of(&payload.aggregate_id)?,
                slug_of(&payload.target, "target")?,
            )),
            IntentExecutionEventDto::Parked(payload) => IntentExecutionEvent::Parked(Parked::new(
                event_id_of(&payload.id)?,
                aggregate_id_of(&payload.aggregate_id)?,
                slug_of(&payload.stage, "stage")?,
            )),
            IntentExecutionEventDto::Unparked(payload) => {
                IntentExecutionEvent::Unparked(Unparked::new(
                    event_id_of(&payload.id)?,
                    aggregate_id_of(&payload.aggregate_id)?,
                ))
            }
            IntentExecutionEventDto::Recomposed(payload) => {
                IntentExecutionEvent::Recomposed(Recomposed::new(
                    event_id_of(&payload.id)?,
                    aggregate_id_of(&payload.aggregate_id)?,
                    slugs_of(&payload.skipped, "skipped")?,
                    slugs_of(&payload.added, "added")?,
                ))
            }
            IntentExecutionEventDto::AutonomyModeSet(payload) => {
                IntentExecutionEvent::AutonomyModeSet(AutonomyModeSet::new(
                    event_id_of(&payload.id)?,
                    aggregate_id_of(&payload.aggregate_id)?,
                    autonomy_of(&payload.mode)?,
                ))
            }
            IntentExecutionEventDto::SingleStageRunCommitted(payload) => {
                IntentExecutionEvent::SingleStageRunCommitted(SingleStageRunCommitted::new(
                    event_id_of(&payload.id)?,
                    aggregate_id_of(&payload.aggregate_id)?,
                    slug_of(&payload.stage, "stage")?,
                ))
            }
            IntentExecutionEventDto::SkeletonStanceRecorded(payload) => {
                IntentExecutionEvent::SkeletonStanceRecorded(SkeletonStanceRecorded::new(
                    event_id_of(&payload.id)?,
                    aggregate_id_of(&payload.aggregate_id)?,
                    skeleton_stance_of(&payload.stance, "stance")?,
                ))
            }
            IntentExecutionEventDto::ReviewRequested(payload) => {
                IntentExecutionEvent::ReviewRequested(ReviewRequested::new(
                    event_id_of(&payload.id)?,
                    aggregate_id_of(&payload.aggregate_id)?,
                    slug_of(&payload.stage, "stage")?,
                    payload.reviewer.clone(),
                    payload.iteration,
                    payload.retry,
                ))
            }
            IntentExecutionEventDto::ReviewCompleted(payload) => {
                IntentExecutionEvent::ReviewCompleted(ReviewCompleted::new(
                    event_id_of(&payload.id)?,
                    aggregate_id_of(&payload.aggregate_id)?,
                    slug_of(&payload.stage, "stage")?,
                    payload.reviewer.clone(),
                    payload.iteration,
                    review_verdict_of(&payload.verdict, "verdict")?,
                ))
            }
            IntentExecutionEventDto::PracticesAffirmed(payload) => {
                IntentExecutionEvent::PracticesAffirmed(PracticesAffirmed::new(
                    event_id_of(&payload.id)?,
                    aggregate_id_of(&payload.aggregate_id)?,
                    slug_of(&payload.stage, "stage")?,
                    payload.affirming_user.clone(),
                    payload
                        .sections
                        .iter()
                        .map(PromotedSectionDto::to_domain)
                        .collect(),
                    payload.mandated.clone(),
                    payload.forbidden.clone(),
                ))
            }
        })
    }
}
