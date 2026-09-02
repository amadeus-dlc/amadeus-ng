//! `DefinitionContentDto` — 定義の内容 (3 入力のモデル) の行の形 (**読む側**)。
//!
//! 誕生 ([`DefinedDto`]) と改訂 ([`RedefinedDto`]) はどちらもこの形を運ぶ。内容の綴りが
//! 1 か所に束なるので、面ごとの乖離が構造的に起きない。
//!
//! # 内容に従属する行の形は同居する
//!
//! ステージ・消費宣言・規則・センサ参照・スコープメタデータの 5 つは、いずれもこの内容に
//! **従属する非公開型**なので同じファイルに納める — 非公開型だけの孤立ファイルを作らない
//! (`coding-rules/abstract-data-type.md` §改訂 2026-09-01)。読む側の `IntentDto` (誕生材料と
//! その部品 4 型) や、書く側の `workflow_definition_dto.rs` と同じ形であり、側ごと複製の
//! 読み比べもファイル単位でできる。
//!
//! 閉集合の綴りは [`dto_vocabulary`](super::dto_vocabulary) が持つ。ドメインの
//! `as_str` / `parse` は流用しない — 同じ値でも面ごとに綴りが違うからである
//! (例: `ExecutionKind` はジャーナル上 `"Always"` だが `stage-graph.json` 上は `"ALWAYS"`)。
//!
//! [`DefinedDto`]: super::defined_dto::DefinedDto
//! [`RedefinedDto`]: super::redefined_dto::RedefinedDto

use std::collections::BTreeMap;

use core_command_domain::workflow_definition::{
    ConsumeDecl, RuleInContext, ScopeGrid, ScopeMetadata, SensorRef, StageGraph, StageNode,
    StageNodeBuilder, StageNumber, StageSlug,
};
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::dto_vocabulary::{
    execution_kind_of, execution_kind_spelling, phase_of, phase_spelling, plan_action_of,
    plan_action_spelling, project_type_of, project_type_spelling, review_cap_of,
    review_cap_spelling, review_class_of, review_class_spelling, rule_scope_of,
    rule_scope_spelling, skeleton_default_of, skeleton_default_spelling, stage_mode_of,
    stage_mode_spelling,
};

/// 定義の内容 (3 入力のモデル) の行の形。**フィールド名と並びが契約**である。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct DefinitionContentDto {
    /// 文書順のステージ列 (読込時に数値順へ正規化しない — F2)。
    graph: Vec<StageNodeDto>,
    /// `<scope> -> <slug> -> EXECUTE|SKIP`。**`stage-graph.json` 面の中間 `"stages"` キーは
    /// 持たない** — あれはレガシー互換のための 2 段構造であって、行の形ではない。
    grid: BTreeMap<String, BTreeMap<String, String>>,
    /// スコープカタログ。各要素が自分の名前を持つので、鍵で二重に持たない。
    scopes: Vec<ScopeMetadataDto>,
}

/// ステージ 1 件の行の形 (28 フィールド)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StageNodeDto {
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

/// 上流成果物の消費宣言の行の形。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ConsumeDeclDto {
    artifact: String,
    required: bool,
    conditional_on: Option<String>,
}

/// 文脈に載る規則 1 件の行の形。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RuleInContextDto {
    path: String,
    scope: String,
}

/// センサ参照 1 件の行の形。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SensorRefDto {
    id: String,
    path: String,
    matches: Option<String>,
}

/// スコープメタデータ 1 件の行の形。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ScopeMetadataDto {
    name: String,
    depth: Option<String>,
    keywords: Vec<String>,
    skeleton: Option<String>,
    review_cap: Option<String>,
    freeform_default: bool,
}

impl DefinitionContentDto {
    /// ドメインの 3 入力から DTO を組む (書き — テストが行を用意する口)。
    pub(super) fn of(
        graph: &StageGraph,
        grid: &ScopeGrid,
        scopes: &BTreeMap<String, ScopeMetadata>,
    ) -> DefinitionContentDto {
        DefinitionContentDto {
            graph: graph.nodes().iter().map(StageNodeDto::of).collect(),
            grid: grid
                .columns()
                .iter()
                .map(|(scope, cells)| {
                    (
                        scope.clone(),
                        cells
                            .iter()
                            .map(|(slug, action)| {
                                (
                                    slug.as_str().to_string(),
                                    plan_action_spelling(*action).to_string(),
                                )
                            })
                            .collect(),
                    )
                })
                .collect(),
            scopes: scopes.values().map(ScopeMetadataDto::of).collect(),
        }
    }

    /// ドメインの 3 入力へ戻す (読み)。
    ///
    /// # Errors
    ///
    /// 閉集合外の綴り・文法外の識別子は `Malformed`、グラフの不変条件違反 (slug 重複など)
    /// は `InvariantViolation`。
    pub(super) fn to_domain(
        &self,
    ) -> Result<(StageGraph, ScopeGrid, BTreeMap<String, ScopeMetadata>), DtoDecodeError> {
        let mut nodes = Vec::with_capacity(self.graph.len());
        for node in &self.graph {
            nodes.push(node.to_domain()?);
        }
        let graph = StageGraph::new(nodes).map_err(|_| DtoDecodeError::InvariantViolation)?;

        let mut columns = BTreeMap::new();
        for (scope, cells) in &self.grid {
            let mut column = BTreeMap::new();
            for (slug, action) in cells {
                column.insert(
                    StageSlug::parse(slug)
                        .map_err(|_| DtoDecodeError::malformed("grid_slug", slug.clone()))?,
                    plan_action_of(action, "grid_action")?,
                );
            }
            columns.insert(scope.clone(), column);
        }

        let mut scopes = BTreeMap::new();
        for scope in &self.scopes {
            let metadata = scope.to_domain()?;
            scopes.insert(metadata.name().to_string(), metadata);
        }
        Ok((graph, ScopeGrid::new(columns), scopes))
    }
}

impl StageNodeDto {
    fn of(node: &StageNode) -> StageNodeDto {
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

    fn to_domain(&self) -> Result<StageNode, DtoDecodeError> {
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

impl ConsumeDeclDto {
    fn of(decl: &ConsumeDecl) -> ConsumeDeclDto {
        ConsumeDeclDto {
            artifact: decl.artifact().to_string(),
            required: decl.required(),
            conditional_on: decl
                .conditional_on()
                .map(|value| project_type_spelling(value).to_string()),
        }
    }

    fn to_domain(&self) -> Result<ConsumeDecl, DtoDecodeError> {
        let conditional_on = match &self.conditional_on {
            None => None,
            Some(raw) => Some(project_type_of(raw)?),
        };
        Ok(ConsumeDecl::new(
            self.artifact.clone(),
            self.required,
            conditional_on,
        ))
    }
}

impl RuleInContextDto {
    fn of(rule: &RuleInContext) -> RuleInContextDto {
        RuleInContextDto {
            path: rule.path().to_string(),
            scope: rule_scope_spelling(rule.scope()).to_string(),
        }
    }

    fn to_domain(&self) -> Result<RuleInContext, DtoDecodeError> {
        Ok(RuleInContext::new(
            self.path.clone(),
            rule_scope_of(&self.scope)?,
        ))
    }
}

impl SensorRefDto {
    fn of(sensor: &SensorRef) -> SensorRefDto {
        SensorRefDto {
            id: sensor.id().to_string(),
            path: sensor.path().to_string(),
            matches: sensor.matches().map(str::to_string),
        }
    }

    /// 閉集合を持たないので失敗しない。
    fn to_domain(&self) -> SensorRef {
        SensorRef::new(self.id.clone(), self.path.clone(), self.matches.clone())
    }
}

impl ScopeMetadataDto {
    fn of(metadata: &ScopeMetadata) -> ScopeMetadataDto {
        ScopeMetadataDto {
            name: metadata.name().to_string(),
            depth: metadata.depth().map(str::to_string),
            keywords: metadata.keywords().to_vec(),
            skeleton: metadata
                .skeleton()
                .map(|value| skeleton_default_spelling(value).to_string()),
            review_cap: metadata
                .review_cap()
                .map(|value| review_cap_spelling(value).to_string()),
            freeform_default: metadata.freeform_default(),
        }
    }

    fn to_domain(&self) -> Result<ScopeMetadata, DtoDecodeError> {
        let mut metadata = ScopeMetadata::new(&self.name)
            .map_err(|_| DtoDecodeError::malformed("scope_name", self.name.clone()))?;
        if let Some(depth) = &self.depth {
            metadata = metadata.with_depth(depth.clone());
        }
        if !self.keywords.is_empty() {
            metadata = metadata.with_keywords(self.keywords.clone());
        }
        if let Some(skeleton) = &self.skeleton {
            metadata = metadata.with_skeleton(skeleton_default_of(skeleton)?);
        }
        if let Some(review_cap) = &self.review_cap {
            metadata = metadata.with_review_cap(review_cap_of(review_cap)?);
        }
        Ok(metadata.with_freeform_default(self.freeform_default))
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    reason = "想定外ケースの即時失敗はテストの検証手段である (house style)"
)]
mod tests {
    use super::super::definition_dto_tests::{content, saturated_node, saturated_scopes};
    use super::*;
    use core_command_domain::workflow_definition::{
        BrownfieldGreenfield, ReviewCapValue, ReviewClass, RuleScope, SkeletonDefault,
    };

    /// 飽和スコープカタログの唯一の要素。
    fn saturated_scope() -> ScopeMetadata {
        saturated_scopes()
            .remove("feature")
            .expect("飽和カタログは feature を持つ")
    }

    // ---- 内容 (3 入力) ----

    #[test]
    fn the_three_inputs_survive_the_round_trip() {
        let (graph, grid, scopes) = content();
        let (decoded_graph, decoded_grid, decoded_scopes) =
            DefinitionContentDto::of(&graph, &grid, &scopes)
                .to_domain()
                .unwrap();
        assert_eq!(decoded_graph, graph);
        assert_eq!(decoded_grid, grid);
        assert_eq!(decoded_scopes, scopes);
    }

    #[test]
    fn a_grid_cell_that_cannot_be_carried_into_the_domain_is_refused() {
        // グリッドの鍵 (slug) と値 (Execute/Skip) はどちらも閉集合である。
        let (graph, grid, scopes) = content();
        let sound = DefinitionContentDto::of(&graph, &grid, &scopes);

        let mut bad_slug = sound.clone();
        bad_slug.grid = [(
            "feature".to_string(),
            [("Bad Slug".to_string(), "Execute".to_string())]
                .into_iter()
                .collect(),
        )]
        .into_iter()
        .collect();
        assert_eq!(
            bad_slug.to_domain().unwrap_err(),
            DtoDecodeError::malformed("grid_slug", "Bad Slug")
        );

        let mut bad_action = sound;
        bad_action.grid = [(
            "feature".to_string(),
            [("code-generation".to_string(), "EXECUTE".to_string())]
                .into_iter()
                .collect(),
        )]
        .into_iter()
        .collect();
        assert_eq!(
            bad_action.to_domain().unwrap_err(),
            DtoDecodeError::malformed("grid_action", "EXECUTE")
        );
    }

    #[test]
    fn a_graph_that_breaks_its_invariant_is_refused_as_an_invariant_violation() {
        // slug の重複はグラフの不変条件違反 — 綴りの問題ではないので材料は欄名を持たない。
        let (graph, grid, scopes) = content();
        let mut dto = DefinitionContentDto::of(&graph, &grid, &scopes);
        let duplicated = dto.graph.first().cloned().unwrap();
        dto.graph.push(duplicated);
        assert_eq!(
            dto.to_domain().unwrap_err(),
            DtoDecodeError::InvariantViolation
        );
    }

    // ---- ステージ ----

    #[test]
    fn every_optional_stage_field_survives_the_round_trip() {
        let node = saturated_node();
        let decoded = StageNodeDto::of(&node).to_domain().unwrap();
        assert_eq!(decoded, node);
        assert_eq!(decoded.plugin(), Some("acme"));
        assert_eq!(decoded.enabled(), Some(false), "3 値をそのまま運ぶ");
        assert_eq!(decoded.review_class(), Some(ReviewClass::Adversarial));
    }

    #[test]
    fn an_unknown_stage_spelling_is_refused_field_by_field() {
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

    // ---- 消費宣言 ----

    #[test]
    fn the_declaration_survives_the_round_trip() {
        let decl = ConsumeDecl::new("design", true, Some(BrownfieldGreenfield::Brownfield));
        assert_eq!(ConsumeDeclDto::of(&decl).to_domain().unwrap(), decl);

        let bare = ConsumeDecl::new("design", false, None);
        assert_eq!(ConsumeDeclDto::of(&bare).to_domain().unwrap(), bare);
    }

    #[test]
    fn an_unknown_project_type_spelling_is_refused() {
        let mut dto = ConsumeDeclDto::of(&ConsumeDecl::new("design", true, None));
        dto.conditional_on = Some("Brownfield".to_string());
        assert_eq!(
            dto.to_domain().unwrap_err(),
            DtoDecodeError::malformed("project_type", "Brownfield")
        );
    }

    // ---- 文脈に載る規則 ----

    #[test]
    fn the_rule_survives_the_round_trip() {
        for scope in [
            RuleScope::Org,
            RuleScope::Team,
            RuleScope::Project,
            RuleScope::Phase,
        ] {
            let rule = RuleInContext::new("org.md", scope);
            assert_eq!(RuleInContextDto::of(&rule).to_domain().unwrap(), rule);
        }
    }

    #[test]
    fn an_unknown_rule_scope_spelling_is_refused() {
        let mut dto = RuleInContextDto::of(&RuleInContext::new("org.md", RuleScope::Org));
        dto.scope = "org".to_string();
        assert_eq!(
            dto.to_domain().unwrap_err(),
            DtoDecodeError::malformed("rule_scope", "org")
        );
    }

    // ---- センサ参照 ----

    #[test]
    fn the_sensor_reference_survives_the_round_trip() {
        let sensor = SensorRef::new("linter", "sensors/linter.md", Some("*.rs".to_string()));
        assert_eq!(SensorRefDto::of(&sensor).to_domain(), sensor);

        let bare = SensorRef::new("linter", "sensors/linter.md", None);
        assert_eq!(SensorRefDto::of(&bare).to_domain(), bare);
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "契約 JSON ではなくワイヤ形式そのものの逐語固定 (BR1.7 の射程外)"
    )]
    #[test]
    fn the_absent_match_pattern_stays_absent() {
        // `matches` は「宣言が無い」を `None` で表す — 空文字列へ潰さない。
        let dto = SensorRefDto::of(&SensorRef::new("linter", "sensors/linter.md", None));
        assert_eq!(
            serde_json::to_string(&dto).unwrap(),
            r#"{"id":"linter","path":"sensors/linter.md","matches":null}"#
        );
    }

    // ---- スコープメタデータ ----

    #[test]
    fn every_optional_scope_field_survives_the_round_trip() {
        let metadata = saturated_scope();
        assert_eq!(
            ScopeMetadataDto::of(&metadata).to_domain().unwrap(),
            metadata
        );

        let bare = ScopeMetadata::new("feature").unwrap();
        assert_eq!(ScopeMetadataDto::of(&bare).to_domain().unwrap(), bare);
    }

    #[test]
    fn an_empty_scope_name_is_refused_as_malformed() {
        let mut dto = ScopeMetadataDto::of(&saturated_scope());
        dto.name = String::new();
        assert_eq!(
            dto.to_domain().unwrap_err(),
            DtoDecodeError::malformed("scope_name", "")
        );
    }

    #[test]
    fn an_unknown_scope_metadata_spelling_is_refused_field_by_field() {
        let mut skeleton = ScopeMetadataDto::of(&saturated_scope());
        skeleton.skeleton = Some("on".to_string());
        assert_eq!(
            skeleton.to_domain().unwrap_err(),
            DtoDecodeError::malformed("skeleton", "on")
        );

        let mut cap = ScopeMetadataDto::of(&saturated_scope());
        cap.review_cap = Some("advisory".to_string());
        assert_eq!(
            cap.to_domain().unwrap_err(),
            DtoDecodeError::malformed("review_cap", "advisory")
        );

        // 飽和フィクスチャがこの 2 値を実際に持っていることを前提にしている。
        assert_eq!(saturated_scope().skeleton(), Some(SkeletonDefault::On));
        assert_eq!(
            saturated_scope().review_cap(),
            Some(ReviewCapValue::Advisory)
        );
    }
}
