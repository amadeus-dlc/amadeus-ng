//! ドメインイベント 12 変種の永続化 DTO — ジャーナル行 `payload` 列のバイト形 (**読む側**)。
//!
//! 外部タグ付き列挙 (`{"Started": { .. }}`) で、`Unparked` だけが材料を持たない単位変種
//! (`"Unparked"`) である。**変種名・フィールド名・並びが契約**である。
//!
//! 各変種の材料 DTO は自分専用のファイルに 1 型 1 ファイルで置き (`one-public-type`)、
//! `of` / `to_domain` もそれぞれの型が持つ。ここでの `of` / `to_domain` は各変種への
//! 委譲だけを行う (`coding-rules/abstract-data-type.md` — 1 ファイル 1 公開型)。

use core_command_domain::orchestration::IntentExecutionEvent;
use core_command_domain::workflow_definition::StageSlug;
use serde::{Deserialize, Serialize};

use super::autonomy_mode_set_dto::AutonomyModeSetDto;
use super::dto_decode_error::DtoDecodeError;
use super::gate_approved_dto::GateApprovedDto;
use super::gate_opened_dto::GateOpenedDto;
use super::gate_rejected_dto::GateRejectedDto;
use super::jumped_dto::JumpedDto;
use super::parked_dto::ParkedDto;
use super::recomposed_dto::RecomposedDto;
use super::stage_completed_dto::StageCompletedDto;
use super::stage_revised_dto::StageRevisedDto;
use super::stage_skipped_dto::StageSkippedDto;
use super::started_dto::StartedDto;

/// ジャーナル行 `payload` の形。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentExecutionEventDto {
    /// 実行の開始 (解決済み計画を自己完結で持つ)。
    Started(StartedDto),
    /// 非ゲートステージの完了。
    StageCompleted(StageCompletedDto),
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
    /// park マーカーの除去 (材料なし)。
    Unparked,
    /// 実効プランの再形成。
    Recomposed(RecomposedDto),
    /// 自律モードの設定。
    AutonomyModeSet(AutonomyModeSetDto),
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
            IntentExecutionEvent::StageCompleted(payload) => {
                IntentExecutionEventDto::StageCompleted(StageCompletedDto::of(payload))
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
            IntentExecutionEvent::Unparked => IntentExecutionEventDto::Unparked,
            IntentExecutionEvent::Recomposed(payload) => {
                IntentExecutionEventDto::Recomposed(RecomposedDto::of(payload))
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
            IntentExecutionEventDto::StageCompleted(payload) => {
                IntentExecutionEvent::StageCompleted(payload.to_domain()?)
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
            IntentExecutionEventDto::Unparked => IntentExecutionEvent::Unparked,
            IntentExecutionEventDto::Recomposed(payload) => {
                IntentExecutionEvent::Recomposed(payload.to_domain()?)
            }
            IntentExecutionEventDto::AutonomyModeSet(payload) => {
                IntentExecutionEvent::AutonomyModeSet(payload.to_domain()?)
            }
        })
    }
}
