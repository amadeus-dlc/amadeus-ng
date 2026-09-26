//! `read_definition_scope_phase_entry` の行を組む投影 — スコープ × フェーズの入口を
//! [`DefinitionScopePhaseEntryRow`] へ写す。

use core_command_domain::workflow_definition::{PhaseId, StageNode, WorkflowDefinitionId};

use super::row_id;
use crate::orchestration::DefinitionScopePhaseEntryRow;

/// スコープ × フェーズの入口を 1 行へ写す。
///
/// `first` は定義のクエリ `first_in_scope_stage_of_phase` の答えである。答えが `None` の
/// フェーズには呼出側が行を作らない。
pub(super) fn row(
    definition_id: &WorkflowDefinitionId,
    scope: &str,
    phase: PhaseId,
    first: &StageNode,
) -> DefinitionScopePhaseEntryRow {
    DefinitionScopePhaseEntryRow::new(
        row_id::definition_scope_phase_entry(definition_id.as_str(), scope, phase.as_str()),
        definition_id.as_str().to_string(),
        scope.to_string(),
        phase.as_str().to_string(),
        first.slug().as_str().to_string(),
    )
}
