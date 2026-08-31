//! ドメインイベント 12 変種の永続化 DTO — ジャーナル行 `payload` 列のバイト形。
//!
//! 外部タグ付き列挙 (`{"Started": { .. }}`) で、`Unparked` だけが材料を持たない単位変種
//! (`"Unparked"`) である。**変種名・フィールド名・並びが契約**である。

use core_command_domain::orchestration::{
    AutonomyModeSet, GateApproved, GateOpened, GateRejected, IntentExecutionEvent, IntentId,
    Jumped, Parked, Recomposed, StageCompleted, StageRevised, StageSkipped, Started,
};
use core_command_domain::workflow_definition::StageSlug;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::dto_vocabulary::{autonomy_of, autonomy_spelling};

/// ジャーナル行 `payload` の形。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentExecutionEventDto {
    /// 実行の開始 (事実の主体 = intent の識別子だけ — issue #56)。
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

/// `Started` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartedDto {
    intent_id: String,
}

/// `StageCompleted` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageCompletedDto {
    stage: String,
}

/// `GateOpened` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateOpenedDto {
    stage: String,
    artifacts: Vec<String>,
}

/// `GateApproved` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateApprovedDto {
    stage: String,
    user_input: Option<String>,
}

/// `GateRejected` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateRejectedDto {
    stage: String,
    feedback: Option<String>,
}

/// `StageRevised` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageRevisedDto {
    stage: String,
}

/// `StageSkipped` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageSkippedDto {
    stage: String,
    reason: String,
}

/// `Jumped` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JumpedDto {
    target: String,
}

/// `Parked` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParkedDto {
    stage: String,
}

/// `Recomposed` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecomposedDto {
    skipped: Vec<String>,
    added: Vec<String>,
}

/// `AutonomyModeSet` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomyModeSetDto {
    mode: String,
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
                    intent_id: payload.intent_id().as_str().to_string(),
                })
            }
            IntentExecutionEvent::StageCompleted(payload) => {
                IntentExecutionEventDto::StageCompleted(StageCompletedDto {
                    stage: slug_spelling(payload.stage()),
                })
            }
            IntentExecutionEvent::GateOpened(payload) => {
                IntentExecutionEventDto::GateOpened(GateOpenedDto {
                    stage: slug_spelling(payload.stage()),
                    artifacts: payload.artifacts().to_vec(),
                })
            }
            IntentExecutionEvent::GateApproved(payload) => {
                IntentExecutionEventDto::GateApproved(GateApprovedDto {
                    stage: slug_spelling(payload.stage()),
                    user_input: payload.user_input().map(str::to_string),
                })
            }
            IntentExecutionEvent::GateRejected(payload) => {
                IntentExecutionEventDto::GateRejected(GateRejectedDto {
                    stage: slug_spelling(payload.stage()),
                    feedback: payload.feedback().map(str::to_string),
                })
            }
            IntentExecutionEvent::StageRevised(payload) => {
                IntentExecutionEventDto::StageRevised(StageRevisedDto {
                    stage: slug_spelling(payload.stage()),
                })
            }
            IntentExecutionEvent::StageSkipped(payload) => {
                IntentExecutionEventDto::StageSkipped(StageSkippedDto {
                    stage: slug_spelling(payload.stage()),
                    reason: payload.reason().to_string(),
                })
            }
            IntentExecutionEvent::Jumped(payload) => IntentExecutionEventDto::Jumped(JumpedDto {
                target: slug_spelling(payload.target()),
            }),
            IntentExecutionEvent::Parked(payload) => IntentExecutionEventDto::Parked(ParkedDto {
                stage: slug_spelling(payload.stage()),
            }),
            IntentExecutionEvent::Unparked => IntentExecutionEventDto::Unparked,
            IntentExecutionEvent::Recomposed(payload) => {
                IntentExecutionEventDto::Recomposed(RecomposedDto {
                    skipped: payload.skipped().iter().map(slug_spelling).collect(),
                    added: payload.added().iter().map(slug_spelling).collect(),
                })
            }
            IntentExecutionEvent::AutonomyModeSet(payload) => {
                IntentExecutionEventDto::AutonomyModeSet(AutonomyModeSetDto {
                    mode: autonomy_spelling(payload.mode()).to_string(),
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
                IntentExecutionEvent::Started(Started::new(
                    IntentId::parse(&payload.intent_id)
                        .map_err(|_| DtoDecodeError::malformed("intent_id", &payload.intent_id))?,
                ))
            }
            IntentExecutionEventDto::StageCompleted(payload) => {
                IntentExecutionEvent::StageCompleted(StageCompleted::new(slug_of(
                    &payload.stage,
                    "stage",
                )?))
            }
            IntentExecutionEventDto::GateOpened(payload) => IntentExecutionEvent::GateOpened(
                GateOpened::new(slug_of(&payload.stage, "stage")?, payload.artifacts.clone()),
            ),
            IntentExecutionEventDto::GateApproved(payload) => {
                IntentExecutionEvent::GateApproved(GateApproved::new(
                    slug_of(&payload.stage, "stage")?,
                    payload.user_input.clone(),
                ))
            }
            IntentExecutionEventDto::GateRejected(payload) => IntentExecutionEvent::GateRejected(
                GateRejected::new(slug_of(&payload.stage, "stage")?, payload.feedback.clone()),
            ),
            IntentExecutionEventDto::StageRevised(payload) => IntentExecutionEvent::StageRevised(
                StageRevised::new(slug_of(&payload.stage, "stage")?),
            ),
            IntentExecutionEventDto::StageSkipped(payload) => IntentExecutionEvent::StageSkipped(
                StageSkipped::new(slug_of(&payload.stage, "stage")?, payload.reason.clone()),
            ),
            IntentExecutionEventDto::Jumped(payload) => {
                IntentExecutionEvent::Jumped(Jumped::new(slug_of(&payload.target, "target")?))
            }
            IntentExecutionEventDto::Parked(payload) => {
                IntentExecutionEvent::Parked(Parked::new(slug_of(&payload.stage, "stage")?))
            }
            IntentExecutionEventDto::Unparked => IntentExecutionEvent::Unparked,
            IntentExecutionEventDto::Recomposed(payload) => {
                IntentExecutionEvent::Recomposed(Recomposed::new(
                    slugs_of(&payload.skipped, "skipped")?,
                    slugs_of(&payload.added, "added")?,
                ))
            }
            IntentExecutionEventDto::AutonomyModeSet(payload) => {
                IntentExecutionEvent::AutonomyModeSet(AutonomyModeSet::new(autonomy_of(
                    &payload.mode,
                )?))
            }
        })
    }
}
