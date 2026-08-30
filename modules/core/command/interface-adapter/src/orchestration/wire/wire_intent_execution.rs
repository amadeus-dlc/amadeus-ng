//! 集約の永続化 DTO — スナップショット行 `payload` 列のバイト形。
//!
//! 型名は集約の具体名 (`WireIntentExecution`) — スナップショットとは**ある時点の集約
//! そのもの**であり、Snapshot という別概念の型は作らない (オーナー裁定 2026-08-30)。

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    IntentExecution, IntentExecutionId, IntentId, StageIndex, StageKey,
};
use core_command_domain::workflow_definition::StageSlug;
use serde::{Deserialize, Serialize};

use super::wire_error::WireDecodeError;
use super::wire_vocabulary::{
    autonomy_of, autonomy_spelling, checkbox_of, checkbox_spelling, phase_of, phase_spelling,
    plan_action_of, plan_action_spelling, status_of, status_spelling,
};

/// スナップショット行の形。**フィールド名と並びが契約**である。
///
/// 楽観 version は載らない — 版数の正本は本家 v3 の `SnapshotEnvelope::version()` (行の列) で
/// あり、`payload` 列は純粋なドメイン内容だけを持つ (ADR-010)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireIntentExecution {
    id: String,
    intent_id: String,
    /// イベント適用の添字帳 (slug + phase) — 集約の自己完結 replay の材料 (issue #44)。
    stages: Vec<WireStageKey>,
    overlay: Vec<String>,
    checkbox: Vec<String>,
    cursor: usize,
    status: String,
    parked_at: Option<usize>,
    autonomy: String,
    approved: Vec<bool>,
    revision_count: Vec<u32>,
    seq_nr: usize,
    last_updated_at: DateTime<Utc>,
}

/// 添字帳 1 行のワイヤ形。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct WireStageKey {
    slug: String,
    phase: String,
}

impl WireIntentExecution {
    /// 集約の読取面からスナップショット行の形を組む (書き)。
    ///
    /// memento 型は経由しない (オーナー裁定 2026-08-30 — 集約と構造同一の写し型は複製で
    /// しかない)。**フィールド名と並びと綴りは従来と同一**であり、行のバイトは変わらない。
    #[must_use]
    pub fn of(execution: &IntentExecution) -> WireIntentExecution {
        let stages = 0..execution.stage_count();
        WireIntentExecution {
            id: execution.id().as_str().to_string(),
            intent_id: execution.intent_id().as_str().to_string(),
            stages: execution
                .stage_keys()
                .iter()
                .map(|key| WireStageKey {
                    slug: key.slug().as_str().to_string(),
                    phase: phase_spelling(key.phase()).to_string(),
                })
                .collect(),
            overlay: stages
                .clone()
                .filter_map(|value| execution.stage_index(value))
                .filter_map(|stage| execution.effective_plan(stage))
                .map(|action| plan_action_spelling(action).to_string())
                .collect(),
            checkbox: stages
                .clone()
                .filter_map(|value| execution.stage_index(value))
                .filter_map(|stage| execution.checkbox(stage))
                .map(|state| checkbox_spelling(state).to_string())
                .collect(),
            cursor: execution.cursor().to_usize(),
            status: status_spelling(execution.status()).to_string(),
            parked_at: execution.parked_at().map(StageIndex::to_usize),
            autonomy: autonomy_spelling(execution.autonomy()).to_string(),
            approved: stages
                .clone()
                .filter_map(|value| execution.stage_index(value))
                .filter_map(|stage| execution.approved(stage))
                .collect(),
            revision_count: stages
                .filter_map(|value| execution.stage_index(value))
                .filter_map(|stage| execution.revision_count(stage))
                .collect(),
            seq_nr: execution.seq_nr(),
            last_updated_at: *execution.last_updated_at(),
        }
    }

    /// 行から集約へ戻す (読み — 集約の完全コンストラクタ [`IntentExecution::new`] を必ず通る)。
    ///
    /// # Errors
    ///
    /// 綴りの復号失敗・識別子の文法違反・集約不変条件の違反 (いずれも呼出側が `Corrupt` へ
    /// 写す — BR1.5)。
    pub fn to_domain(&self) -> Result<IntentExecution, WireDecodeError> {
        let id = IntentExecutionId::parse(&self.id)
            .map_err(|_| WireDecodeError::malformed("id", &self.id))?;
        let intent_id = IntentId::parse(&self.intent_id)
            .map_err(|_| WireDecodeError::malformed("intent_id", &self.intent_id))?;
        let stage_keys = self
            .stages
            .iter()
            .map(|key| {
                Ok(StageKey::new(
                    StageSlug::parse(&key.slug)
                        .map_err(|_| WireDecodeError::malformed("stages.slug", &key.slug))?,
                    phase_of(&key.phase, "stages.phase")?,
                ))
            })
            .collect::<Result<Vec<_>, WireDecodeError>>()?;
        let overlay = self
            .overlay
            .iter()
            .map(|raw| plan_action_of(raw, "overlay"))
            .collect::<Result<Vec<_>, WireDecodeError>>()?;
        let checkbox = self
            .checkbox
            .iter()
            .map(|raw| checkbox_of(raw))
            .collect::<Result<Vec<_>, WireDecodeError>>()?;
        IntentExecution::new(
            id,
            intent_id,
            stage_keys,
            overlay,
            checkbox,
            self.cursor,
            status_of(&self.status)?,
            self.parked_at,
            autonomy_of(&self.autonomy)?,
            self.approved.clone(),
            self.revision_count.clone(),
            self.seq_nr,
            self.last_updated_at,
        )
        .map_err(|error| WireDecodeError::malformed("intent_execution", error.reason()))
    }
}
