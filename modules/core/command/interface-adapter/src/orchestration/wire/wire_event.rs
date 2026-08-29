//! ドメインイベント 12 変種の永続化 DTO — ジャーナル行 `payload` 列のバイト形。
//!
//! 外部タグ付き列挙 (`{"Started": { .. }}`) で、`Unparked` だけが材料を持たない単位変種
//! (`"Unparked"`) である。**変種名・フィールド名・並びが契約**である。

use core_command_domain::orchestration::{
    AutonomyModeSet, GateApproved, GateOpened, GateRejected, IntentExecutionEvent, Jumped, Parked,
    PhaseBoundary, Recomposed, StageCompleted, StageRevised, StageSkipped, Started,
};
use core_command_domain::workflow_definition::StageSlug;
use serde::{Deserialize, Serialize};

use super::wire_error::WireDecodeError;
use super::wire_intent::WireIntent;
use super::wire_vocabulary::{
    autonomy_of, autonomy_spelling, direction_of, direction_spelling, phase_of, phase_spelling,
};

/// ジャーナル行 `payload` の形。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireEvent {
    /// 実行の開始 (解決済み計画を自己完結で持つ)。
    Started(WireStarted),
    /// 非ゲートステージの完了。
    StageCompleted(WireStageCompleted),
    /// 承認ゲートの開放。
    GateOpened(WireGateOpened),
    /// 承認ゲートの通過。
    GateApproved(WireGateApproved),
    /// 承認ゲートでの差し戻し。
    GateRejected(WireGateRejected),
    /// 差し戻し後のゲート再入。
    StageRevised(WireStageRevised),
    /// ステージの読み飛ばし。
    StageSkipped(WireStageSkipped),
    /// カーソルの移動。
    Jumped(WireJumped),
    /// park マーカーの設置。
    Parked(WireParked),
    /// park マーカーの除去 (材料なし)。
    Unparked,
    /// 実効プランの再形成。
    Recomposed(WireRecomposed),
    /// 自律モードの設定。
    AutonomyModeSet(WireAutonomyModeSet),
}

/// `Started` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireStarted {
    intent: WireIntent,
}

/// `StageCompleted` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireStageCompleted {
    stage: String,
    next_stage: Option<String>,
}

/// `GateOpened` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireGateOpened {
    stage: String,
    artifacts: Vec<String>,
}

/// `GateApproved` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireGateApproved {
    stage: String,
    user_input: Option<String>,
    next_stage: Option<String>,
    phase_boundary: Option<WirePhaseBoundary>,
}

/// `GateRejected` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireGateRejected {
    stage: String,
    feedback: Option<String>,
    revision_count: u32,
}

/// `StageRevised` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireStageRevised {
    stage: String,
}

/// `StageSkipped` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireStageSkipped {
    stage: String,
    reason: String,
    next_stage: Option<String>,
}

/// `Jumped` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireJumped {
    direction: String,
    source: String,
    target: String,
    stages_reset: Vec<String>,
    stages_skipped: Vec<String>,
}

/// `Parked` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireParked {
    stage: String,
}

/// `Recomposed` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireRecomposed {
    skipped: Vec<String>,
    added: Vec<String>,
    stages_in_scope: Vec<String>,
}

/// `AutonomyModeSet` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireAutonomyModeSet {
    mode: String,
}

/// フェーズ境界の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct WirePhaseBoundary {
    from_phase: String,
    to_phase: String,
}

/// ステージ参照の綴り。
fn slug_spelling(slug: &StageSlug) -> String {
    slug.as_str().to_string()
}

/// ステージ参照の復号。
fn slug_of(raw: &str, field: &'static str) -> Result<StageSlug, WireDecodeError> {
    StageSlug::parse(raw).map_err(|_| WireDecodeError::malformed(field, raw))
}

/// ステージ参照の列の復号。
fn slugs_of(raw: &[String], field: &'static str) -> Result<Vec<StageSlug>, WireDecodeError> {
    raw.iter().map(|value| slug_of(value, field)).collect()
}

/// 省略可能なステージ参照の復号。
fn optional_slug_of(
    raw: Option<&String>,
    field: &'static str,
) -> Result<Option<StageSlug>, WireDecodeError> {
    raw.map(|value| slug_of(value, field)).transpose()
}

impl WireEvent {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    #[must_use]
    pub fn of(event: &IntentExecutionEvent) -> WireEvent {
        match event {
            IntentExecutionEvent::Started(payload) => WireEvent::Started(WireStarted {
                intent: WireIntent::of(payload.intent()),
            }),
            IntentExecutionEvent::StageCompleted(payload) => {
                WireEvent::StageCompleted(WireStageCompleted {
                    stage: slug_spelling(payload.stage()),
                    next_stage: payload.next_stage().map(slug_spelling),
                })
            }
            IntentExecutionEvent::GateOpened(payload) => WireEvent::GateOpened(WireGateOpened {
                stage: slug_spelling(payload.stage()),
                artifacts: payload.artifacts().to_vec(),
            }),
            IntentExecutionEvent::GateApproved(payload) => {
                WireEvent::GateApproved(WireGateApproved {
                    stage: slug_spelling(payload.stage()),
                    user_input: payload.user_input().map(str::to_string),
                    next_stage: payload.next_stage().map(slug_spelling),
                    phase_boundary: payload.phase_boundary().map(WirePhaseBoundary::of),
                })
            }
            IntentExecutionEvent::GateRejected(payload) => {
                WireEvent::GateRejected(WireGateRejected {
                    stage: slug_spelling(payload.stage()),
                    feedback: payload.feedback().map(str::to_string),
                    revision_count: payload.revision_count(),
                })
            }
            IntentExecutionEvent::StageRevised(payload) => {
                WireEvent::StageRevised(WireStageRevised {
                    stage: slug_spelling(payload.stage()),
                })
            }
            IntentExecutionEvent::StageSkipped(payload) => {
                WireEvent::StageSkipped(WireStageSkipped {
                    stage: slug_spelling(payload.stage()),
                    reason: payload.reason().to_string(),
                    next_stage: payload.next_stage().map(slug_spelling),
                })
            }
            IntentExecutionEvent::Jumped(payload) => WireEvent::Jumped(WireJumped {
                direction: direction_spelling(payload.direction()).to_string(),
                source: slug_spelling(payload.source()),
                target: slug_spelling(payload.target()),
                stages_reset: payload.stages_reset().iter().map(slug_spelling).collect(),
                stages_skipped: payload.stages_skipped().iter().map(slug_spelling).collect(),
            }),
            IntentExecutionEvent::Parked(payload) => WireEvent::Parked(WireParked {
                stage: slug_spelling(payload.stage()),
            }),
            IntentExecutionEvent::Unparked => WireEvent::Unparked,
            IntentExecutionEvent::Recomposed(payload) => WireEvent::Recomposed(WireRecomposed {
                skipped: payload.skipped().iter().map(slug_spelling).collect(),
                added: payload.added().iter().map(slug_spelling).collect(),
                stages_in_scope: payload
                    .stages_in_scope()
                    .iter()
                    .map(slug_spelling)
                    .collect(),
            }),
            IntentExecutionEvent::AutonomyModeSet(payload) => {
                WireEvent::AutonomyModeSet(WireAutonomyModeSet {
                    mode: autonomy_spelling(payload.mode()).to_string(),
                })
            }
        }
    }

    /// ドメインイベントへ戻す (読み)。
    ///
    /// # Errors
    ///
    /// 閉集合外の綴り・文法外のステージ参照は `Malformed`、`Started` が運ぶ intent が
    /// Always Valid を破る場合は `InvariantViolation` を返す。
    pub fn to_domain(&self) -> Result<IntentExecutionEvent, WireDecodeError> {
        Ok(match self {
            WireEvent::Started(payload) => {
                IntentExecutionEvent::Started(Started::new(payload.intent.to_domain()?))
            }
            WireEvent::StageCompleted(payload) => {
                IntentExecutionEvent::StageCompleted(StageCompleted::new(
                    slug_of(&payload.stage, "stage")?,
                    optional_slug_of(payload.next_stage.as_ref(), "next_stage")?,
                ))
            }
            WireEvent::GateOpened(payload) => IntentExecutionEvent::GateOpened(GateOpened::new(
                slug_of(&payload.stage, "stage")?,
                payload.artifacts.clone(),
            )),
            WireEvent::GateApproved(payload) => {
                IntentExecutionEvent::GateApproved(GateApproved::new(
                    slug_of(&payload.stage, "stage")?,
                    payload.user_input.clone(),
                    optional_slug_of(payload.next_stage.as_ref(), "next_stage")?,
                    payload
                        .phase_boundary
                        .as_ref()
                        .map(WirePhaseBoundary::to_domain)
                        .transpose()?,
                ))
            }
            WireEvent::GateRejected(payload) => {
                IntentExecutionEvent::GateRejected(GateRejected::new(
                    slug_of(&payload.stage, "stage")?,
                    payload.feedback.clone(),
                    payload.revision_count,
                ))
            }
            WireEvent::StageRevised(payload) => IntentExecutionEvent::StageRevised(
                StageRevised::new(slug_of(&payload.stage, "stage")?),
            ),
            WireEvent::StageSkipped(payload) => {
                IntentExecutionEvent::StageSkipped(StageSkipped::new(
                    slug_of(&payload.stage, "stage")?,
                    payload.reason.clone(),
                    optional_slug_of(payload.next_stage.as_ref(), "next_stage")?,
                ))
            }
            WireEvent::Jumped(payload) => IntentExecutionEvent::Jumped(Jumped::new(
                direction_of(&payload.direction)?,
                slug_of(&payload.source, "source")?,
                slug_of(&payload.target, "target")?,
                slugs_of(&payload.stages_reset, "stages_reset")?,
                slugs_of(&payload.stages_skipped, "stages_skipped")?,
            )),
            WireEvent::Parked(payload) => {
                IntentExecutionEvent::Parked(Parked::new(slug_of(&payload.stage, "stage")?))
            }
            WireEvent::Unparked => IntentExecutionEvent::Unparked,
            WireEvent::Recomposed(payload) => IntentExecutionEvent::Recomposed(Recomposed::new(
                slugs_of(&payload.skipped, "skipped")?,
                slugs_of(&payload.added, "added")?,
                slugs_of(&payload.stages_in_scope, "stages_in_scope")?,
            )),
            WireEvent::AutonomyModeSet(payload) => IntentExecutionEvent::AutonomyModeSet(
                AutonomyModeSet::new(autonomy_of(&payload.mode)?),
            ),
        })
    }
}

impl WirePhaseBoundary {
    fn of(boundary: PhaseBoundary) -> WirePhaseBoundary {
        WirePhaseBoundary {
            from_phase: phase_spelling(boundary.from_phase()).to_string(),
            to_phase: phase_spelling(boundary.to_phase()).to_string(),
        }
    }

    fn to_domain(&self) -> Result<PhaseBoundary, WireDecodeError> {
        Ok(PhaseBoundary::new(
            phase_of(&self.from_phase, "from_phase")?,
            phase_of(&self.to_phase, "to_phase")?,
        ))
    }
}
