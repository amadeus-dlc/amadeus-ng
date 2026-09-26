//! `read_intent_stage` の行を組む投影 — 解決済み計画のステージ 1 件を [`IntentStageRow`] へ
//! 写す。

use core_command_domain::orchestration::{IntentId, StageEntry};

use super::row_id;
use crate::orchestration::IntentStageRow;

/// 計画のステージ 1 件を 1 行へ写す。
pub(super) fn row(intent_id: &IntentId, stage_index: usize, entry: &StageEntry) -> IntentStageRow {
    IntentStageRow::new(
        row_id::intent_stage(intent_id.as_str(), stage_index),
        intent_id.as_str().to_string(),
        stage_index,
        entry.slug().as_str().to_string(),
        entry.phase().as_str().to_string(),
        entry.plan_action().as_str().to_string(),
        entry.is_conditional(),
        entry.display().number().as_str().to_string(),
        entry.display().name().to_string(),
        entry.display().lead_agent().to_string(),
        entry.is_gated(),
    )
}
