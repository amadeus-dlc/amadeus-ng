//! `read_definition_scope_keyword` の行を組む投影 — 語とその先着スコープを
//! [`DefinitionScopeKeywordRow`] へ写す。

use core_command_domain::workflow_definition::WorkflowDefinitionId;

use super::row_id;
use crate::orchestration::DefinitionScopeKeywordRow;

/// 語とその先着スコープを 1 行へ写す。
///
/// どのスコープが先着かは呼出側が辞書順の畳み込みで決める。ここはその答えを写すだけである。
pub(super) fn row(
    definition_id: &WorkflowDefinitionId,
    keyword: &str,
    scope: &str,
) -> DefinitionScopeKeywordRow {
    DefinitionScopeKeywordRow::new(
        row_id::definition_scope_keyword(definition_id.as_str(), keyword),
        definition_id.as_str().to_string(),
        keyword.to_string(),
        scope.to_string(),
    )
}
