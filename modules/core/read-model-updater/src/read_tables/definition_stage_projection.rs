//! `read_definition_stage` の行を組む投影 — グラフのノード 1 件を [`DefinitionStageRow`] へ
//! 全列で写す。

use core_command_domain::orchestration::StageKey;
use core_command_domain::workflow_definition::{ReviewClass, StageNode, WorkflowDefinitionId};

use super::json_column;
use super::row_id;
use crate::orchestration::DefinitionStageRow;

/// グラフのノード 1 件を 1 行へ写す。
///
/// `position` は文書順 (グラフの並びそのもの)。`gated` はドメインの述語
/// [`StageKey::is_gated`] に問う — 「initialization だけが非ゲート」という規則を
/// ここで書き直さない。
pub(super) fn row(
    definition_id: &WorkflowDefinitionId,
    position: usize,
    node: &StageNode,
) -> DefinitionStageRow {
    let key = StageKey::new(node.slug().clone(), node.phase());
    DefinitionStageRow::new(
        row_id::definition_stage(definition_id.as_str(), node.slug().as_str()),
        definition_id.as_str().to_string(),
        node.slug().as_str().to_string(),
        position,
        node.number().as_str().to_string(),
        node.name().to_string(),
        node.phase().as_str().to_string(),
        node.execution().as_str().to_string(),
        node.condition().to_string(),
        node.lead_agent().to_string(),
        json_column::strings(node.support_agents()),
        node.mode().as_str().to_string(),
        node.for_each().map(str::to_string),
        node.workspace_requires(),
        json_column::strings(node.produces()),
        json_column::strings(node.optional_produces()),
        json_column::produces_kinds(node.produces_kinds()),
        json_column::consumes(node.consumes()),
        json_column::slugs(node.requires_stage()),
        json_column::strings(node.sensors()),
        json_column::strings(node.scopes()),
        node.reviewer().map(str::to_string),
        node.reviewer_max_iterations(),
        node.review_class()
            .map(ReviewClass::as_str)
            .map(str::to_string),
        node.summary_confirmation().map(str::to_string),
        node.plugin().map(str::to_string),
        node.enabled(),
        key.is_gated(),
        node.inputs().to_string(),
        node.outputs().to_string(),
        json_column::rules_in_context(node.rules_in_context()),
        json_column::sensors_applicable(node.sensors_applicable()),
    )
}
