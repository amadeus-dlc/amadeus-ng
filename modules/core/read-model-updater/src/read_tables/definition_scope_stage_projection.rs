//! `read_definition_scope_stage` の行を組む投影 — スコープ × ステージの 1 セルを
//! [`DefinitionScopeStageRow`] へ写す。

use core_command_domain::workflow_definition::{PlanAction, StageSlug, WorkflowDefinitionId};

use super::row_id;
use crate::orchestration::DefinitionScopeStageRow;

/// スコープ × ステージの 1 セルを 1 行へ写す。
///
/// `action` は定義のクエリ `stages_in_scope` の答えで、答えが無ければ NULL のまま写す
/// (EXECUTE や SKIP へ丸めない)。`in_scope_order` は呼出側が EXECUTE のセルにだけ振った
/// 文書順の連番である。
pub(super) fn row(
    definition_id: &WorkflowDefinitionId,
    scope: &str,
    stage_slug: &StageSlug,
    action: Option<PlanAction>,
    in_scope_order: Option<usize>,
) -> DefinitionScopeStageRow {
    DefinitionScopeStageRow::new(
        row_id::definition_scope_stage(definition_id.as_str(), scope, stage_slug.as_str()),
        definition_id.as_str().to_string(),
        scope.to_string(),
        stage_slug.as_str().to_string(),
        action.map(PlanAction::as_str).map(str::to_string),
        in_scope_order,
    )
}
