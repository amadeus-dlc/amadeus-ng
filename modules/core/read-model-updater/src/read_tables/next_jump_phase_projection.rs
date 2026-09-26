//! `read_next_jump_phase` の行を組む投影 — `--phase` ジャンプの目的地を [`NextJumpPhaseRow`]
//! へ写す。

use core_command_domain::orchestration::{IntentExecution, StageIndex};
use core_command_domain::workflow_definition::PhaseId;

use super::row_id;
use super::stage_lookup::slug_of;
use crate::orchestration::NextJumpPhaseRow;

/// 1 つのフェーズの目的地を 1 行へ写す。
///
/// `target` は集約のクエリ `first_in_scope_of_phase` の答えである。答えが `None` の
/// フェーズには呼出側が行を作らない。
pub(super) fn row(
    execution: &IntentExecution,
    phase: PhaseId,
    target: StageIndex,
) -> NextJumpPhaseRow {
    NextJumpPhaseRow::new(
        row_id::next_jump_phase(execution.id().as_str(), phase.as_str()),
        execution.id().as_str().to_string(),
        phase.as_str().to_string(),
        target.to_usize(),
        slug_of(execution, target.to_usize()),
    )
}
