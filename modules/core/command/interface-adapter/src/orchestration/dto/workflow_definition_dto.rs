//! `WorkflowDefinition` とその部品の永続化 DTO — 定義ジャーナル・スナップショットのバイト形。
//!
//! 誕生記録 ([`DefinedDto`]) が「確立された定義の全内容」を張り、スナップショット行
//! ([`WorkflowDefinitionDto`]) はそれに**封筒由来のメタデータ** (`last_updated_at`) を
//! 足した形である (`serde(flatten)` なので行のキーは平坦のまま)。内容の綴りが 1 か所に
//! 束なるので、面ごとの乖離が構造的に起きない。改訂イベント `Redefined` は系譜 ID を
//! 持たないぶんだけ狭く、内容部分 ([`DefinitionContentDto`]) を共有する。
//!
//! スナップショット行だけが時刻を持つのは、**発生時刻がイベントの材料ではなく封筒の
//! メタデータ**だからである。ジャーナル行の時刻は封筒が運ぶので payload に重複させない
//! 一方、スナップショット封筒 (`SnapshotEnvelope`) は `seq_nr` と `version` しか持たないので、
//! 集約の `last_updated_at` は行の内容として書くしかない (`IntentExecutionDto` と同型)。
//!
//! 永続化 DTO は `*Dto` を名乗り `dto/` に置く — `wire` 語は 2026-09-01 のオーナー裁定で全廃した。
//!
//! # ドメインの `as_str` / `parse` は流用しない
//!
//! 閉集合の綴りは [`dto_vocabulary`](super::dto_vocabulary) が持つ。同じ値でも面ごとに
//! 綴りが違うからである (例: `ExecutionKind` はジャーナル上 `"Always"` だが
//! `stage-graph.json` 上は `"ALWAYS"`)。流用すると片方の綴りを変えた瞬間にもう片方のバイトが
//! 壊れる。

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use core_command_domain::workflow_definition::{
    ConsumeDecl, Defined, DefinitionRevision, RuleInContext, ScopeGrid, ScopeMetadata, SensorRef,
    StageGraph, StageNode, StageNodeBuilder, StageNumber, StageSlug, WorkflowDefinition,
    WorkflowDefinitionId,
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

/// スナップショット行の形。**フィールド名と並びが契約**である。
///
/// 通番は載せない — 基底の通番はスナップショット**封筒の列**から来る (`IntentDto` と同じ形)。
/// 最終更新時刻だけは封筒が持たないので行に書く (モジュール doc 参照)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowDefinitionDto {
    /// 誕生記録と同じ内容 — 行のキーは平坦のまま (`flatten`)。
    #[serde(flatten)]
    defined: DefinedDto,
    /// 最後に適用したイベントの発生時刻 (封筒由来のメタデータ)。
    last_updated_at: DateTime<Utc>,
}

/// 誕生記録の行の形 — 系譜 ID・内容版・内容。
///
/// 誕生イベント `Defined` の payload であり、スナップショット行の内容部分でもある。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinedDto {
    id: String,
    revision: String,
    content: DefinitionContentDto,
}

/// 定義の内容 (3 入力のモデル) の行の形。誕生と改訂の両方が同じ形を運ぶ。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct DefinitionContentDto {
    /// 文書順のステージ列 (読込時に数値順へ正規化しない — F2)。
    graph: Vec<StageNodeDto>,
    /// `<scope> -> <slug> -> EXECUTE|SKIP`。**`stage-graph.json` 面の中間 `"stages"` キーは
    /// 持たない** — あれはレガシー互換のための 2 段構造であって、我々の行の形ではない。
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
    produces_kinds: BTreeMap<String, Vec<String>>,
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

impl WorkflowDefinitionDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    #[must_use]
    pub fn of(definition: &WorkflowDefinition) -> WorkflowDefinitionDto {
        WorkflowDefinitionDto {
            defined: DefinedDto {
                id: definition.id().as_str().to_string(),
                revision: definition.revision().as_str().to_string(),
                content: DefinitionContentDto::of(
                    definition.graph(),
                    definition.grid(),
                    definition.scopes(),
                ),
            },
            last_updated_at: *definition.last_updated_at(),
        }
    }

    /// 検査付き再構成経路へ渡してドメインへ戻す (読み)。
    ///
    /// 誕生記録の変換 (`From<(Defined, DateTime<Utc>)>`) を通るので、検査を迂回する構築口は
    /// 存在しない。通番はここでは刻まない — 呼出側 (Repository) が封筒の列から刻む。
    ///
    /// # Errors
    ///
    /// 閉集合外の綴り・文法外の識別子は `Malformed`、グラフの不変条件違反 (slug 重複など)
    /// は `InvariantViolation` を返す。
    pub fn to_domain(&self) -> Result<WorkflowDefinition, DtoDecodeError> {
        Ok(WorkflowDefinition::from((
            self.defined.to_domain()?,
            self.last_updated_at,
        )))
    }
}

impl DefinedDto {
    /// 誕生記録から DTO を組む (書き)。
    #[must_use]
    pub(super) fn of(defined: &Defined) -> DefinedDto {
        DefinedDto {
            id: defined.id().as_str().to_string(),
            revision: defined.revision().as_str().to_string(),
            content: DefinitionContentDto::of(defined.graph(), defined.grid(), defined.scopes()),
        }
    }

    /// 誕生記録として復号する (読み — 定義ジャーナル面・スナップショット面の共通経路)。
    ///
    /// # Errors
    ///
    /// 閉集合外の綴り・文法外の識別子・不変条件違反。
    pub(super) fn to_domain(&self) -> Result<Defined, DtoDecodeError> {
        let (graph, grid, scopes) = self.content.to_domain()?;
        Ok(Defined::new(
            WorkflowDefinitionId::parse(&self.id)
                .map_err(|_| DtoDecodeError::malformed("id", self.id.clone()))?,
            DefinitionRevision::parse(&self.revision)
                .map_err(|_| DtoDecodeError::malformed("revision", self.revision.clone()))?,
            graph,
            grid,
            scopes,
        ))
    }
}

impl DefinitionContentDto {
    /// ドメインの 3 入力から DTO を組む (書き)。
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
            produces_kinds: node.produces_kinds().clone(),
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
mod tests {
    #![allow(
        clippy::indexing_slicing,
        reason = "固定長フィクスチャの添字参照 (house style — dto/tests.rs と同じ)"
    )]

    use super::*;
    use core_command_domain::workflow_definition::{
        BrownfieldGreenfield, DefinitionRevision, ExecutionKind, PhaseId, ReviewCapValue,
        ReviewClass, RuleScope, SkeletonDefault, StageMode, StageNodeBuilder, WorkflowDefinition,
    };

    /// スナップショット行に載る発生時刻 (固定値)。
    fn at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-08-31T00:00:00Z")
            .expect("固定の ISO 8601 UTC")
            .with_timezone(&Utc)
    }

    /// **任意フィールドを 1 つ残らず埋めた**ステージ。
    ///
    /// 配布物 (`stage-graph.json`) には `plugin` も `enabled` も現れないので、ゴールデン
    /// パリティの往復ではこの 2 つの写像が通らない。行のバイトを決めるのはこの層なので、
    /// 「出荷データに出ないフィールド」こそ意図的に埋めて往復させる。
    fn saturated_node() -> StageNode {
        StageNodeBuilder::new(
            StageSlug::parse("code-generation").expect("slug"),
            StageNumber::parse("3.1").expect("番号"),
            "Code Generation".to_string(),
            PhaseId::Construction,
            ExecutionKind::Conditional,
            StageMode::Mob,
        )
        .condition("brownfield".to_string())
        .lead_agent("developer".to_string())
        .support_agents(vec!["quality".to_string()])
        .for_each("unit".to_string())
        .workspace_requires(true)
        .produces(vec!["code".to_string()])
        .optional_produces(vec!["notes".to_string()])
        .produces_kinds(
            [("code".to_string(), vec!["rust".to_string()])]
                .into_iter()
                .collect(),
        )
        .consumes(vec![ConsumeDecl::new(
            "design",
            true,
            Some(BrownfieldGreenfield::Brownfield),
        )])
        .requires_stage(vec![StageSlug::parse("domain-design").expect("slug")])
        .sensors(vec!["linter".to_string()])
        .scopes(vec!["feature".to_string()])
        .reviewer("architecture-reviewer".to_string())
        .reviewer_max_iterations(2)
        .review_class(ReviewClass::Adversarial)
        .summary_confirmation("required".to_string())
        .plugin("acme".to_string())
        .enabled(false)
        .inputs("design".to_string())
        .outputs("code".to_string())
        .rules_in_context(vec![RuleInContext::new("org.md", RuleScope::Org)])
        .sensors_applicable(vec![SensorRef::new(
            "linter",
            "sensors/linter.md",
            Some("*.rs".to_string()),
        )])
        .build()
    }

    /// **任意フィールドを 1 つ残らず埋めた**スコープメタデータ。
    fn saturated_scopes() -> BTreeMap<String, ScopeMetadata> {
        let metadata = ScopeMetadata::new("feature")
            .expect("スコープ名")
            .with_depth("standard".to_string())
            .with_keywords(vec!["api".to_string(), "endpoint".to_string()])
            .with_skeleton(SkeletonDefault::On)
            .with_review_cap(ReviewCapValue::Advisory)
            .with_freeform_default(true);
        [("feature".to_string(), metadata)].into_iter().collect()
    }

    fn saturated_definition() -> WorkflowDefinition {
        let graph = StageGraph::new(vec![saturated_node()]).expect("グラフ");
        let grid = ScopeGrid::from_graph(&graph);
        WorkflowDefinition::define(
            WorkflowDefinitionId::parse("claude").expect("定義 id"),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision"),
            graph,
            grid,
            saturated_scopes(),
            at(),
        )
        .0
    }

    #[test]
    fn every_optional_field_survives_the_round_trip() {
        // 出荷データに現れないフィールド (plugin / enabled / depth / keywords / skeleton /
        // review_cap) まで含めて往復する。どれか 1 つが DTO から落ちればここで割れる。
        let definition = saturated_definition();
        let dto = WorkflowDefinitionDto::of(&definition);
        let decoded = dto.to_domain().expect("往復できる");
        assert_eq!(decoded, definition);

        let node = decoded.graph().at(0).expect("1 ノード");
        assert_eq!(node.plugin(), Some("acme"));
        assert_eq!(node.enabled(), Some(false), "3 値をそのまま運ぶ");
        assert_eq!(node.review_class(), Some(ReviewClass::Adversarial));
        assert_eq!(node.mode(), StageMode::Mob);
        assert_eq!(node.execution(), ExecutionKind::Conditional);
        assert_eq!(
            node.consumes()[0].conditional_on(),
            Some(BrownfieldGreenfield::Brownfield)
        );
        assert_eq!(node.rules_in_context()[0].scope(), RuleScope::Org);

        let scope = decoded.scope_metadata("feature").expect("スコープ");
        assert_eq!(scope.depth(), Some("standard"));
        assert_eq!(
            scope.keywords(),
            ["api".to_string(), "endpoint".to_string()]
        );
        assert_eq!(scope.skeleton(), Some(SkeletonDefault::On));
        assert_eq!(scope.review_cap(), Some(ReviewCapValue::Advisory));
        assert!(scope.freeform_default());
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "契約 JSON ではなくワイヤ形式そのものの逐語固定 (BR1.7 の射程外)"
    )]
    #[test]
    fn the_snapshot_row_carries_the_time_beside_the_birth_record_keys() {
        // スナップショット封筒は seq_nr と version しか持たないので、集約の
        // `last_updated_at` は**行の内容**として書くしかない (`IntentExecutionDto` と同型)。
        // `flatten` なので誕生記録の 3 キーは入れ子にならない。
        let json = serde_json::to_string(&WorkflowDefinitionDto::of(&saturated_definition()))
            .expect("DTO は直列化できる");

        assert!(json.starts_with(r#"{"id":"claude","revision":"#), "{json}");
        assert!(
            json.contains(r#""last_updated_at":"2026-08-31T00:00:00Z""#),
            "{json}"
        );
        assert!(!json.contains(r#""defined":"#), "入れ子にしない: {json}");
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "契約 JSON ではなくワイヤ形式そのものの逐語固定 (BR1.7 の射程外)"
    )]
    #[test]
    fn a_row_with_a_broken_spelling_is_refused_field_by_field() {
        // 閉集合外の綴り・文法外の識別子は、どの欄で落ちたかを材料に載せて拒む。
        // ワイヤ形式の JSON を直接壊す — 実装に破壊用のフックを開けない。
        let json = serde_json::to_string(&WorkflowDefinitionDto::of(&saturated_definition()))
            .expect("DTO は直列化できる");
        for (from, to, field) in [
            (r#""id":"claude""#, r#""id":"  ""#, "id"),
            (r#""revision":"sha256:"#, r#""revision":"nope:"#, "revision"),
            (r#""mode":"Mob""#, r#""mode":"mob""#, "mode"),
            (
                r#""execution":"Conditional""#,
                r#""execution":"CONDITIONAL""#,
                "execution",
            ),
            (
                r#""phase":"Construction""#,
                r#""phase":"construction""#,
                "phase",
            ),
            (
                r#""review_class":"Adversarial""#,
                r#""review_class":"adversarial""#,
                "review_class",
            ),
            (r#""scope":"Org""#, r#""scope":"org""#, "rule_scope"),
            (
                r#""conditional_on":"brownfield""#,
                r#""conditional_on":"Brownfield""#,
                "project_type",
            ),
            (r#""skeleton":"On""#, r#""skeleton":"on""#, "skeleton"),
            (
                r#""review_cap":"Advisory""#,
                r#""review_cap":"advisory""#,
                "review_cap",
            ),
            (r#""name":"feature""#, r#""name":"""#, "scope_name"),
            (r#""number":"3.1""#, r#""number":"three""#, "number"),
            (
                r#""slug":"code-generation""#,
                r#""slug":"Code Generation""#,
                "slug",
            ),
            (
                r#""requires_stage":["domain-design"]"#,
                r#""requires_stage":["Domain Design"]"#,
                "requires_stage",
            ),
        ] {
            let broken = json.replacen(from, to, 1);
            assert_ne!(broken, json, "置換が効いていない: {from}");
            let dto: WorkflowDefinitionDto =
                serde_json::from_str(&broken).expect("形は DTO として読める");
            let error = dto.to_domain().expect_err("綴りが違えばドメインへ戻せない");
            assert!(
                error
                    .to_string()
                    .starts_with(&format!("malformed field {field}")),
                "{field}: {error}"
            );
        }
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "契約 JSON ではなくワイヤ形式そのものの逐語固定 (BR1.7 の射程外)"
    )]
    #[test]
    fn a_grid_cell_that_cannot_be_carried_into_the_domain_is_refused() {
        // グリッドの鍵 (slug) と値 (EXECUTE/SKIP) はどちらも閉集合である。
        let json = serde_json::to_string(&WorkflowDefinitionDto::of(&saturated_definition()))
            .expect("DTO は直列化できる");
        for (from, to, field) in [
            (
                r#"{"code-generation":"Execute"}"#,
                r#"{"Bad Slug":"Execute"}"#,
                "grid_slug",
            ),
            (
                r#"{"code-generation":"Execute"}"#,
                r#"{"code-generation":"EXECUTE"}"#,
                "grid_action",
            ),
        ] {
            let broken = json.replacen(from, to, 1);
            assert_ne!(broken, json, "置換が効いていない: {from}");
            let dto: WorkflowDefinitionDto =
                serde_json::from_str(&broken).expect("形は DTO として読める");
            let error = dto.to_domain().expect_err("グリッドの綴りも検査する");
            assert!(
                error
                    .to_string()
                    .starts_with(&format!("malformed field {field}")),
                "{field}: {error}"
            );
        }
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "契約 JSON ではなくワイヤ形式そのものの逐語固定 (BR1.7 の射程外)"
    )]
    #[test]
    fn a_graph_that_breaks_its_invariant_is_refused_as_an_invariant_violation() {
        // slug の重複はグラフの不変条件違反 — 綴りの問題ではないので材料は欄名を持たない。
        let json = serde_json::to_string(&WorkflowDefinitionDto::of(&saturated_definition()))
            .expect("DTO は直列化できる");
        let node_start = json
            .find(r#"{"slug":"code-generation""#)
            .expect("ノードが在る");
        let node_end = json.find(r#"],"grid""#).expect("グラフ配列の終端");
        let node = json
            .get(node_start..node_end)
            .expect("ノードの範囲")
            .to_string();
        let duplicated = json.replacen(&node, &format!("{node},{node}"), 1);

        let dto: WorkflowDefinitionDto =
            serde_json::from_str(&duplicated).expect("形は DTO として読める");
        assert_eq!(
            dto.to_domain().expect_err("slug の重複は不変条件違反"),
            DtoDecodeError::InvariantViolation
        );
    }
}
