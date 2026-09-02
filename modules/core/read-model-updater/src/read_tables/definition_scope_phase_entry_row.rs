//! `DefinitionScopePhaseEntryRow` — `read_definition_scope_phase_entry` の 1 行
//! (スコープ × フェーズの入口ステージ)。

use core_command_domain::workflow_definition::{PhaseId, StageNode, WorkflowDefinitionId};

/// `read_definition_scope_phase_entry` の 1 行。主キーは (`definition_id`, `scope`, `phase`)。
///
/// 値は [`WorkflowDefinition::first_in_scope_stage_of_phase`] の答えである。答えが `None`
/// のフェーズには**行を作らない** — 「無い」を NULL 行で表すと、読取側が「行が無い」と
/// 「行は在るが値が NULL」を区別しなければならなくなる。
///
/// [`WorkflowDefinition::first_in_scope_stage_of_phase`]: core_command_domain::workflow_definition::WorkflowDefinition::first_in_scope_stage_of_phase
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionScopePhaseEntryRow {
    definition_id: String,
    scope: String,
    phase: String,
    first_stage_slug: String,
}

impl DefinitionScopePhaseEntryRow {
    /// スコープ × フェーズの入口を 1 行へ写す (**この型の唯一の構築経路**)。
    #[must_use]
    pub fn of(
        definition_id: &WorkflowDefinitionId,
        scope: &str,
        phase: PhaseId,
        first: &StageNode,
    ) -> DefinitionScopePhaseEntryRow {
        DefinitionScopePhaseEntryRow {
            definition_id: definition_id.as_str().to_string(),
            scope: scope.to_string(),
            phase: phase.as_str().to_string(),
            first_stage_slug: first.slug().as_str().to_string(),
        }
    }

    /// 定義の系譜 ID。
    #[must_use]
    pub fn definition_id(&self) -> &str {
        &self.definition_id
    }

    /// スコープ名。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// フェーズの綴り (`PhaseId::as_str`)。
    #[must_use]
    pub fn phase(&self) -> &str {
        &self.phase
    }

    /// そのフェーズで最初に実行される in-scope ステージの slug。
    #[must_use]
    pub fn first_stage_slug(&self) -> &str {
        &self.first_stage_slug
    }
}
