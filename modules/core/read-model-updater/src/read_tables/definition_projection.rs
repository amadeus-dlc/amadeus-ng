//! `read_definition` の行を組む投影 — 再生した定義を [`DefinitionRow`] へ写す。

use core_command_domain::workflow_definition::WorkflowDefinition;

use crate::orchestration::DefinitionRow;

/// 再生した定義を 1 行へ写す。
pub(super) fn row(definition: &WorkflowDefinition) -> DefinitionRow {
    DefinitionRow::new(
        definition.id().as_str().to_string(),
        definition.revision().as_str().to_string(),
        definition.graph().len(),
        definition.scopes().len(),
    )
}
