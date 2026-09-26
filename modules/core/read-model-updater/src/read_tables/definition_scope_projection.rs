//! `read_definition_scope` の行を組む投影 — スコープ 1 件のメタデータと費用を
//! [`DefinitionScopeRow`] へ写す。

use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, ReviewCapValue, ScopeCost, ScopeMetadata, SkeletonDefault,
    WorkflowDefinition,
};

use super::json_column;
use super::row_id;
use crate::orchestration::DefinitionScopeRow;

/// スコープ 1 件を 1 行へ写す。
///
/// 費用は定義のクエリ `scope_cost` の答えを brownfield / greenfield の両方で写す。
/// グリッド列を持たない有効スコープでは答えが `None` になり、費用列は NULL になる。
pub(super) fn row(
    definition: &WorkflowDefinition,
    scope: &str,
    metadata: &ScopeMetadata,
) -> DefinitionScopeRow {
    let cost = definition.scope_cost(scope, BrownfieldGreenfield::Brownfield);
    let greenfield_cost = definition.scope_cost(scope, BrownfieldGreenfield::Greenfield);
    DefinitionScopeRow::new(
        row_id::definition_scope(definition.id().as_str(), scope),
        definition.id().as_str().to_string(),
        scope.to_string(),
        metadata.depth().map(str::to_string),
        json_column::strings(metadata.keywords()),
        metadata
            .skeleton()
            .map(SkeletonDefault::as_str)
            .map(str::to_string),
        metadata
            .review_cap()
            .map(ReviewCapValue::as_str)
            .map(str::to_string),
        metadata.freeform_default(),
        definition.grid().contains_scope(scope),
        cost.as_ref().map(ScopeCost::total),
        cost.as_ref().map(ScopeCost::execute),
        cost.as_ref().map(ScopeCost::gates),
        cost.as_ref().map(ScopeCost::per_unit_stages),
        greenfield_cost.as_ref().map(ScopeCost::total),
        greenfield_cost.as_ref().map(ScopeCost::execute),
        greenfield_cost.as_ref().map(ScopeCost::gates),
        greenfield_cost.as_ref().map(ScopeCost::per_unit_stages),
    )
}
