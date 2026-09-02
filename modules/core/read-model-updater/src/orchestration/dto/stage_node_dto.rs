//! `StageNode` の永続化 DTO (**読む側**) — ステージ 1 件の行の形 (28 フィールド)。
//!
//! 閉集合の綴りは [`dto_vocabulary`](super::dto_vocabulary) が持つ。ドメインの
//! `as_str` / `parse` は流用しない — 同じ値でも面ごとに綴りが違うからである
//! (例: `ExecutionKind` はジャーナル上 `"Always"` だが `stage-graph.json` 上は `"ALWAYS"`)。

use core_command_domain::workflow_definition::{
    StageNode, StageNodeBuilder, StageNumber, StageSlug,
};
use serde::{Deserialize, Serialize};

use super::consume_decl_dto::ConsumeDeclDto;
use super::dto_decode_error::DtoDecodeError;
use super::dto_vocabulary::{
    execution_kind_of, execution_kind_spelling, phase_of, phase_spelling, review_class_of,
    review_class_spelling, stage_mode_of, stage_mode_spelling,
};
use super::rule_in_context_dto::RuleInContextDto;
use super::sensor_ref_dto::SensorRefDto;

/// ステージ 1 件の行の形 (28 フィールド)。**フィールド名と並びが契約**である。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct StageNodeDto {
    slug: String,
    number: String,
    name: String,
    phase: String,
    execution: String,
    mode: String,
    condition: String,
    lead_agent: String,
    support_agents: Vec<String>,
    for_each: Option<String>,
    workspace_requires: bool,
    produces: Vec<String>,
    optional_produces: Vec<String>,
    #[serde(with = "super::kinds_codec")]
    produces_kinds: Vec<(String, Vec<String>)>,
    consumes: Vec<ConsumeDeclDto>,
    requires_stage: Vec<String>,
    sensors: Vec<String>,
    scopes: Vec<String>,
    reviewer: Option<String>,
    reviewer_max_iterations: Option<u32>,
    review_class: Option<String>,
    summary_confirmation: Option<String>,
    plugin: Option<String>,
    /// `None` = キー不在 = 有効 (12 §3.1)。3 値をそのまま運ぶ。
    enabled: Option<bool>,
    inputs: String,
    outputs: String,
    rules_in_context: Vec<RuleInContextDto>,
    sensors_applicable: Vec<SensorRefDto>,
}

impl StageNodeDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き — テストが行を用意する口)。
    pub(super) fn of(node: &StageNode) -> StageNodeDto {
        StageNodeDto {
            slug: node.slug().as_str().to_string(),
            number: node.number().as_str().to_string(),
            name: node.name().to_string(),
            phase: phase_spelling(node.phase()).to_string(),
            execution: execution_kind_spelling(node.execution()).to_string(),
            mode: stage_mode_spelling(node.mode()).to_string(),
            condition: node.condition().to_string(),
            lead_agent: node.lead_agent().to_string(),
            support_agents: node.support_agents().to_vec(),
            for_each: node.for_each().map(str::to_string),
            workspace_requires: node.workspace_requires(),
            produces: node.produces().to_vec(),
            optional_produces: node.optional_produces().to_vec(),
            produces_kinds: node.produces_kinds().to_vec(),
            consumes: node.consumes().iter().map(ConsumeDeclDto::of).collect(),
            requires_stage: node
                .requires_stage()
                .iter()
                .map(|slug| slug.as_str().to_string())
                .collect(),
            sensors: node.sensors().to_vec(),
            scopes: node.scopes().to_vec(),
            reviewer: node.reviewer().map(str::to_string),
            reviewer_max_iterations: node.reviewer_max_iterations(),
            review_class: node
                .review_class()
                .map(|class| review_class_spelling(class).to_string()),
            summary_confirmation: node.summary_confirmation().map(str::to_string),
            plugin: node.plugin().map(str::to_string),
            enabled: node.enabled(),
            inputs: node.inputs().to_string(),
            outputs: node.outputs().to_string(),
            rules_in_context: node
                .rules_in_context()
                .iter()
                .map(RuleInContextDto::of)
                .collect(),
            sensors_applicable: node
                .sensors_applicable()
                .iter()
                .map(SensorRefDto::of)
                .collect(),
        }
    }

    /// 検査付き再構成経路へ渡してドメインへ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<StageNode, DtoDecodeError> {
        let mut consumes = Vec::with_capacity(self.consumes.len());
        for decl in &self.consumes {
            consumes.push(decl.to_domain()?);
        }
        let mut requires_stage = Vec::with_capacity(self.requires_stage.len());
        for slug in &self.requires_stage {
            requires_stage.push(
                StageSlug::parse(slug)
                    .map_err(|_| DtoDecodeError::malformed("requires_stage", slug.clone()))?,
            );
        }
        let mut rules_in_context = Vec::with_capacity(self.rules_in_context.len());
        for rule in &self.rules_in_context {
            rules_in_context.push(rule.to_domain()?);
        }

        let mut builder = StageNodeBuilder::new(
            StageSlug::parse(&self.slug)
                .map_err(|_| DtoDecodeError::malformed("slug", self.slug.clone()))?,
            StageNumber::parse(&self.number)
                .map_err(|_| DtoDecodeError::malformed("number", self.number.clone()))?,
            self.name.clone(),
            phase_of(&self.phase, "phase")?,
            execution_kind_of(&self.execution)?,
            stage_mode_of(&self.mode)?,
        )
        .condition(self.condition.clone())
        .lead_agent(self.lead_agent.clone())
        .support_agents(self.support_agents.clone())
        .workspace_requires(self.workspace_requires)
        .produces(self.produces.clone())
        .optional_produces(self.optional_produces.clone())
        .produces_kinds(self.produces_kinds.clone())
        .consumes(consumes)
        .requires_stage(requires_stage)
        .sensors(self.sensors.clone())
        .scopes(self.scopes.clone())
        .inputs(self.inputs.clone())
        .outputs(self.outputs.clone())
        .rules_in_context(rules_in_context)
        .sensors_applicable(
            self.sensors_applicable
                .iter()
                .map(SensorRefDto::to_domain)
                .collect(),
        );
        if let Some(value) = &self.for_each {
            builder = builder.for_each(value.clone());
        }
        if let Some(value) = &self.reviewer {
            builder = builder.reviewer(value.clone());
        }
        if let Some(value) = self.reviewer_max_iterations {
            builder = builder.reviewer_max_iterations(value);
        }
        if let Some(value) = &self.review_class {
            builder = builder.review_class(review_class_of(value)?);
        }
        if let Some(value) = &self.summary_confirmation {
            builder = builder.summary_confirmation(value.clone());
        }
        if let Some(value) = &self.plugin {
            builder = builder.plugin(value.clone());
        }
        if let Some(value) = self.enabled {
            builder = builder.enabled(value);
        }
        Ok(builder.build())
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    reason = "想定外ケースの即時失敗はテストの検証手段である (house style)"
)]
mod tests {
    use super::super::definition_dto_tests::saturated_node;
    use super::*;
    use core_command_domain::workflow_definition::ReviewClass;

    #[test]
    fn every_optional_field_survives_the_round_trip() {
        let node = saturated_node();
        let decoded = StageNodeDto::of(&node).to_domain().unwrap();
        assert_eq!(decoded, node);
        assert_eq!(decoded.plugin(), Some("acme"));
        assert_eq!(decoded.enabled(), Some(false), "3 値をそのまま運ぶ");
        assert_eq!(decoded.review_class(), Some(ReviewClass::Adversarial));
    }

    #[test]
    fn an_unknown_closed_set_spelling_is_refused_field_by_field() {
        for (mutate, field) in [
            (
                Box::new(|dto: &mut StageNodeDto| dto.phase = "construction".to_string())
                    as Box<dyn Fn(&mut StageNodeDto)>,
                "phase",
            ),
            (
                Box::new(|dto: &mut StageNodeDto| dto.execution = "CONDITIONAL".to_string()),
                "execution",
            ),
            (
                Box::new(|dto: &mut StageNodeDto| dto.mode = "mob".to_string()),
                "mode",
            ),
            (
                Box::new(|dto: &mut StageNodeDto| {
                    dto.review_class = Some("adversarial".to_string());
                }),
                "review_class",
            ),
        ] {
            let mut dto = StageNodeDto::of(&saturated_node());
            mutate(&mut dto);
            match dto.to_domain().unwrap_err() {
                DtoDecodeError::Malformed { field: got, .. } => assert_eq!(got, field),
                other => panic!("{field}: 綴りの拒否ではない — {other:?}"),
            }
        }
    }

    #[test]
    fn a_grammar_violating_identifier_is_refused() {
        let mut slug = StageNodeDto::of(&saturated_node());
        slug.slug = "Code Generation".to_string();
        assert_eq!(
            slug.to_domain().unwrap_err(),
            DtoDecodeError::malformed("slug", "Code Generation")
        );

        let mut number = StageNodeDto::of(&saturated_node());
        number.number = "three".to_string();
        assert_eq!(
            number.to_domain().unwrap_err(),
            DtoDecodeError::malformed("number", "three")
        );

        let mut requires = StageNodeDto::of(&saturated_node());
        requires.requires_stage = vec!["Domain Design".to_string()];
        assert_eq!(
            requires.to_domain().unwrap_err(),
            DtoDecodeError::malformed("requires_stage", "Domain Design")
        );
    }
}
