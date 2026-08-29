//! 集約の写しの永続化 DTO — スナップショット行 `payload` 列のバイト形。

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{IntentExecution, StageIndex};
use serde::{Deserialize, Serialize};

use super::wire_vocabulary::{
    autonomy_spelling, checkbox_spelling, plan_action_spelling, status_spelling,
};

/// スナップショット行の形。**フィールド名と並びが契約**である。
///
/// 楽観 version は載らない — 版数の正本は本家 v3 の `SnapshotEnvelope::version()` (行の列) で
/// あり、`payload` 列は純粋なドメイン内容だけを持つ (ADR-010)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireSnapshot {
    id: String,
    intent_id: String,
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

impl WireSnapshot {
    /// 集約の読取面からスナップショット行の形を組む (書き)。
    ///
    /// memento 型は経由しない (オーナー裁定 2026-08-30 — 集約と構造同一の写し型は複製で
    /// しかない)。**フィールド名と並びと綴りは従来と同一**であり、行のバイトは変わらない。
    #[must_use]
    pub fn of(execution: &IntentExecution) -> WireSnapshot {
        let stages = 0..execution.stage_count();
        WireSnapshot {
            id: execution.id().as_str().to_string(),
            intent_id: execution.intent_id().as_str().to_string(),
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
}
