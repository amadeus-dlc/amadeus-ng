//! `StageNode` — コンパイル済み `stage-graph.json` の 1 要素 (`FIELD_ORDER` 28 フィールド) の
//! 型付き表現 (レポート §2.2)。
//!
//! 形の観測可能契約 (レポート §5.1-3/4/5):
//! - `rules_in_context` は**オブジェクト配列** `{path, scope}`、`sensors_applicable` も
//!   **オブジェクト配列** `{id, path, matches?}`。**文字列配列に潰さない**
//!   (directive 側の `string[]` は射影であって格納形ではない — `sensor_ids()` /
//!   `rule_paths()` がその射影を明示的に提供する)。
//! - `inputs` / `outputs` は**文字列** (配列にしない)。機械可読な出力は `produces`。
//! - `number` は文字列 `"P.I"` (`StageNumber` が逐語保持)。
//!
//! 相互フィールド不変条件 (`mode ∈ {pipeline, mob} ⇒ support_agents` 非空、
//! `produces_kinds` のキーが `produces ∪ optional_produces` に含まれる等) は upstream では
//! **compile 時**に検証され、ロード経路では再検査されない (レポート §4.3)。本読取モデルも
//! 同様に再検査しない — 「グラフ全体が読めない」と「1 ノードが使えない」の観測差を作らない。

use std::collections::BTreeMap;

use super::execution_kind::ExecutionKind;
use super::phase::PhaseId;
use super::review_class::ReviewClass;
use super::stage_mode::StageMode;
use super::stage_number::StageNumber;
use super::stage_slug::StageSlug;

/// `rules_in_context[].scope` の閉集合 (upstream 08 §110-119)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuleScope {
    Org,
    Team,
    Project,
    Phase,
}

/// 閉集合外の値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownRuleScope(String);

impl UnknownRuleScope {
    #[must_use]
    pub fn new(value: impl Into<String>) -> UnknownRuleScope {
        UnknownRuleScope(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl RuleScope {
    pub const ALL: [RuleScope; 4] = [
        RuleScope::Org,
        RuleScope::Team,
        RuleScope::Project,
        RuleScope::Phase,
    ];

    /// # Errors
    ///
    /// 4 値以外は `UnknownRuleScope` で拒否する。
    pub fn parse(s: &str) -> Result<RuleScope, UnknownRuleScope> {
        Ok(match s {
            "org" => RuleScope::Org,
            "team" => RuleScope::Team,
            "project" => RuleScope::Project,
            "phase" => RuleScope::Phase,
            other => return Err(UnknownRuleScope::new(other)),
        })
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            RuleScope::Org => "org",
            RuleScope::Team => "team",
            RuleScope::Project => "project",
            RuleScope::Phase => "phase",
        }
    }
}

/// compile 時に確定した rules-in-context の 1 エントリ。
///
/// `path` は `default` スペースに**コンパイル時ピン**されており、実行時に再解決しては
/// ならない (レポート §5.1-18)。active space への rebase は別経路の責務。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleInContext {
    path: String,
    scope: RuleScope,
}

impl RuleInContext {
    #[must_use]
    pub fn new(path: impl Into<String>, scope: RuleScope) -> RuleInContext {
        RuleInContext {
            path: path.into(),
            scope,
        }
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub const fn scope(&self) -> RuleScope {
        self.scope
    }
}

/// compile 時に manifest から逐語スナップショットしたセンサー適用宣言。
///
/// フック側は fire 時に manifest を再オープンしない (レポート §2.2 #28)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensorRef {
    id: String,
    path: String,
    /// capability glob の逐語コピー (欠損しうる)。
    matches: Option<String>,
}

impl SensorRef {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        path: impl Into<String>,
        matches: Option<String>,
    ) -> SensorRef {
        SensorRef {
            id: id.into(),
            path: path.into(),
            matches,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// capability glob の逐語コピー (欠損しうる)。
    #[must_use]
    pub fn matches(&self) -> Option<&str> {
        self.matches.as_deref()
    }
}

/// `consumes[].conditional_on` の閉集合。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BrownfieldGreenfield {
    Brownfield,
    Greenfield,
}

/// 閉集合外の値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownBrownfieldGreenfield(String);

impl UnknownBrownfieldGreenfield {
    #[must_use]
    pub fn new(value: impl Into<String>) -> UnknownBrownfieldGreenfield {
        UnknownBrownfieldGreenfield(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl BrownfieldGreenfield {
    pub const ALL: [BrownfieldGreenfield; 2] = [
        BrownfieldGreenfield::Brownfield,
        BrownfieldGreenfield::Greenfield,
    ];

    /// # Errors
    ///
    /// 2 値以外は `UnknownBrownfieldGreenfield` で拒否する。
    pub fn parse(s: &str) -> Result<BrownfieldGreenfield, UnknownBrownfieldGreenfield> {
        Ok(match s {
            "brownfield" => BrownfieldGreenfield::Brownfield,
            "greenfield" => BrownfieldGreenfield::Greenfield,
            other => return Err(UnknownBrownfieldGreenfield::new(other)),
        })
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            BrownfieldGreenfield::Brownfield => "brownfield",
            BrownfieldGreenfield::Greenfield => "greenfield",
        }
    }
}

/// 入力成果物の宣言。`required: false` は欠損しても無言で落ちる (レポート §2.2 #15)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumeDecl {
    artifact: String,
    required: bool,
    conditional_on: Option<BrownfieldGreenfield>,
}

impl ConsumeDecl {
    #[must_use]
    pub fn new(
        artifact: impl Into<String>,
        required: bool,
        conditional_on: Option<BrownfieldGreenfield>,
    ) -> ConsumeDecl {
        ConsumeDecl {
            artifact: artifact.into(),
            required,
            conditional_on,
        }
    }

    /// 成果物の語彙名 (パスではない)。
    #[must_use]
    pub fn artifact(&self) -> &str {
        &self.artifact
    }

    /// `false` は欠損しても無言で落ちる。
    #[must_use]
    pub const fn required(&self) -> bool {
        self.required
    }

    #[must_use]
    pub const fn conditional_on(&self) -> Option<BrownfieldGreenfield> {
        self.conditional_on
    }
}

/// コンパイル済みグラフの 1 ノード。フィールドは `FIELD_ORDER` の 28 エントリに 1:1 対応する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageNode {
    slug: StageSlug,
    number: StageNumber,
    name: String,
    phase: PhaseId,
    execution: ExecutionKind,
    condition: String,
    lead_agent: String,
    support_agents: Vec<String>,
    mode: StageMode,
    for_each: Option<String>,
    workspace_requires: bool,
    produces: Vec<String>,
    optional_produces: Vec<String>,
    produces_kinds: BTreeMap<String, Vec<String>>,
    consumes: Vec<ConsumeDecl>,
    requires_stage: Vec<StageSlug>,
    sensors: Vec<String>,
    scopes: Vec<String>,
    reviewer: Option<String>,
    reviewer_max_iterations: Option<u32>,
    review_class: Option<ReviewClass>,
    summary_confirmation: Option<String>,
    plugin: Option<String>,
    enabled: Option<bool>,
    inputs: String,
    outputs: String,
    rules_in_context: Vec<RuleInContext>,
    sensors_applicable: Vec<SensorRef>,
}

impl StageNode {
    // ---- identity ----

    #[must_use]
    pub const fn slug(&self) -> &StageSlug {
        &self.slug
    }

    #[must_use]
    pub const fn number(&self) -> &StageNumber {
        &self.number
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn phase(&self) -> PhaseId {
        self.phase
    }

    // ---- 適用可否 ----

    #[must_use]
    pub const fn execution(&self) -> ExecutionKind {
        self.execution
    }

    /// 自由記述の適用ルール (人間・LLM 向け。機械可読ではない)。
    #[must_use]
    pub fn condition(&self) -> &str {
        &self.condition
    }

    // ---- ルーティング ----

    #[must_use]
    pub fn lead_agent(&self) -> &str {
        &self.lead_agent
    }

    #[must_use]
    pub fn support_agents(&self) -> &[String] {
        &self.support_agents
    }

    #[must_use]
    pub const fn mode(&self) -> StageMode {
        self.mode
    }

    /// `"unit-of-work"` が付いていれば Unit of Work ごとに反復する。
    #[must_use]
    pub fn for_each(&self) -> Option<&str> {
        self.for_each.as_deref()
    }

    /// ワークスペース実体への書込を要求するか (欠損は `false`)。
    #[must_use]
    pub const fn workspace_requires(&self) -> bool {
        self.workspace_requires
    }

    // ---- 成果物 ----

    /// 成果物の**語彙名** (パスではない)。
    #[must_use]
    pub fn produces(&self) -> &[String] {
        &self.produces
    }

    /// 条件付き成果物。directive の `produces` 解決には**含まれる**。
    #[must_use]
    pub fn optional_produces(&self) -> &[String] {
        &self.optional_produces
    }

    /// 成果物 → 適用 unit kind。**マップに無い成果物は全 kind に適用**される。
    #[must_use]
    pub const fn produces_kinds(&self) -> &BTreeMap<String, Vec<String>> {
        &self.produces_kinds
    }

    #[must_use]
    pub fn consumes(&self) -> &[ConsumeDecl] {
        &self.consumes
    }

    /// 依存ステージ。compile 時に dedup 済みで、`numericOrder(dep) < numericOrder(self)` を満たす。
    #[must_use]
    pub fn requires_stage(&self) -> &[StageSlug] {
        &self.requires_stage
    }

    // ---- センサー / スコープ ----

    /// 著者が pull import したセンサー id。
    #[must_use]
    pub fn sensors(&self) -> &[String] {
        &self.sensors
    }

    /// このステージを EXECUTE にするスコープ名の列挙 (scope-grid の転置元)。
    #[must_use]
    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }

    /// 指定スコープをこのステージが宣言しているか (転置述語の片腕)。
    #[must_use]
    pub fn declares_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope)
    }

    // ---- レビュー ----

    #[must_use]
    pub fn reviewer(&self) -> Option<&str> {
        self.reviewer.as_deref()
    }

    /// 正整数。`reviewer` 宣言が前提 (ADR 0001 決定 4 — 整数は整数型で持つ)。
    #[must_use]
    pub const fn reviewer_max_iterations(&self) -> Option<u32> {
        self.reviewer_max_iterations
    }

    #[must_use]
    pub const fn review_class(&self) -> Option<ReviewClass> {
        self.review_class
    }

    // ---- その他の宣言 ----

    /// 観測値は `"required"` のみ。値域は未確定 (レポート §7) のため文字列で保持する。
    #[must_use]
    pub fn summary_confirmation(&self) -> Option<&str> {
        self.summary_confirmation.as_deref()
    }

    /// 所有プラグイン名 (frontmatter からの逐語コピー)。
    #[must_use]
    pub fn plugin(&self) -> Option<&str> {
        self.plugin.as_deref()
    }

    /// `enabled` の生値。**`None` は「キー不在」= 有効**を意味する (レポート §5.3-5)。
    #[must_use]
    pub const fn enabled(&self) -> Option<bool> {
        self.enabled
    }

    /// 有効なノードか (`None` = 有効、`Some(false)` のみ無効)。
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled != Some(false)
    }

    /// 自由記述の散文 (配列ではない)。
    #[must_use]
    pub fn inputs(&self) -> &str {
        &self.inputs
    }

    /// 自由記述の散文。**記述用途のみで機械可読ではない** (機械可読は `produces`)。
    #[must_use]
    pub fn outputs(&self) -> &str {
        &self.outputs
    }

    // ---- compile 専用フィールド (著者は書けない) ----

    #[must_use]
    pub fn rules_in_context(&self) -> &[RuleInContext] {
        &self.rules_in_context
    }

    #[must_use]
    pub fn sensors_applicable(&self) -> &[SensorRef] {
        &self.sensors_applicable
    }

    // ---- directive 射影 (レポート §2.3 — 格納形と混同しないための明示 API) ----

    /// directive 上の `sensors_applicable` は **id の `string[]`** に潰れる。
    #[must_use]
    pub fn sensor_ids(&self) -> Vec<&str> {
        self.sensors_applicable.iter().map(SensorRef::id).collect()
    }

    /// directive 上の `rules_in_context` は**パスの `string[]`** に潰れる (順序は宣言順)。
    /// transport 時に active space 基準へ上書きされるのは呼出側の責務。
    #[must_use]
    pub fn rule_paths(&self) -> Vec<&str> {
        self.rules_in_context
            .iter()
            .map(RuleInContext::path)
            .collect()
    }
}

/// `StageNode` の唯一の構成経路。
///
/// `new()` の 6 引数は全ノードに必ず存在する型付きコアで、残りは既定値
/// (空コレクション / 空文字列 / `None` / `false`) から setter で積み上げる。
#[derive(Debug, Clone)]
pub struct StageNodeBuilder {
    node: StageNode,
}

impl StageNodeBuilder {
    #[must_use]
    pub const fn new(
        slug: StageSlug,
        number: StageNumber,
        name: String,
        phase: PhaseId,
        execution: ExecutionKind,
        mode: StageMode,
    ) -> StageNodeBuilder {
        StageNodeBuilder {
            node: StageNode {
                slug,
                number,
                name,
                phase,
                execution,
                condition: String::new(),
                lead_agent: String::new(),
                support_agents: Vec::new(),
                mode,
                for_each: None,
                workspace_requires: false,
                produces: Vec::new(),
                optional_produces: Vec::new(),
                produces_kinds: BTreeMap::new(),
                consumes: Vec::new(),
                requires_stage: Vec::new(),
                sensors: Vec::new(),
                scopes: Vec::new(),
                reviewer: None,
                reviewer_max_iterations: None,
                review_class: None,
                summary_confirmation: None,
                plugin: None,
                enabled: None,
                inputs: String::new(),
                outputs: String::new(),
                rules_in_context: Vec::new(),
                sensors_applicable: Vec::new(),
            },
        }
    }

    #[must_use]
    pub fn condition(mut self, condition: String) -> StageNodeBuilder {
        self.node.condition = condition;
        self
    }

    #[must_use]
    pub fn lead_agent(mut self, lead_agent: String) -> StageNodeBuilder {
        self.node.lead_agent = lead_agent;
        self
    }

    #[must_use]
    pub fn support_agents(mut self, support_agents: Vec<String>) -> StageNodeBuilder {
        self.node.support_agents = support_agents;
        self
    }

    #[must_use]
    pub fn for_each(mut self, for_each: String) -> StageNodeBuilder {
        self.node.for_each = Some(for_each);
        self
    }

    #[must_use]
    pub const fn workspace_requires(mut self, workspace_requires: bool) -> StageNodeBuilder {
        self.node.workspace_requires = workspace_requires;
        self
    }

    #[must_use]
    pub fn produces(mut self, produces: Vec<String>) -> StageNodeBuilder {
        self.node.produces = produces;
        self
    }

    #[must_use]
    pub fn optional_produces(mut self, optional_produces: Vec<String>) -> StageNodeBuilder {
        self.node.optional_produces = optional_produces;
        self
    }

    #[must_use]
    pub fn produces_kinds(
        mut self,
        produces_kinds: BTreeMap<String, Vec<String>>,
    ) -> StageNodeBuilder {
        self.node.produces_kinds = produces_kinds;
        self
    }

    #[must_use]
    pub fn consumes(mut self, consumes: Vec<ConsumeDecl>) -> StageNodeBuilder {
        self.node.consumes = consumes;
        self
    }

    #[must_use]
    pub fn requires_stage(mut self, requires_stage: Vec<StageSlug>) -> StageNodeBuilder {
        self.node.requires_stage = requires_stage;
        self
    }

    #[must_use]
    pub fn sensors(mut self, sensors: Vec<String>) -> StageNodeBuilder {
        self.node.sensors = sensors;
        self
    }

    #[must_use]
    pub fn scopes(mut self, scopes: Vec<String>) -> StageNodeBuilder {
        self.node.scopes = scopes;
        self
    }

    #[must_use]
    pub fn reviewer(mut self, reviewer: String) -> StageNodeBuilder {
        self.node.reviewer = Some(reviewer);
        self
    }

    #[must_use]
    pub const fn reviewer_max_iterations(mut self, iterations: u32) -> StageNodeBuilder {
        self.node.reviewer_max_iterations = Some(iterations);
        self
    }

    #[must_use]
    pub const fn review_class(mut self, review_class: ReviewClass) -> StageNodeBuilder {
        self.node.review_class = Some(review_class);
        self
    }

    #[must_use]
    pub fn summary_confirmation(mut self, summary_confirmation: String) -> StageNodeBuilder {
        self.node.summary_confirmation = Some(summary_confirmation);
        self
    }

    #[must_use]
    pub fn plugin(mut self, plugin: String) -> StageNodeBuilder {
        self.node.plugin = Some(plugin);
        self
    }

    /// `enabled` を明示的に立てる。呼ばなければ `None` (= 有効) のまま。
    #[must_use]
    pub const fn enabled(mut self, enabled: bool) -> StageNodeBuilder {
        self.node.enabled = Some(enabled);
        self
    }

    #[must_use]
    pub fn inputs(mut self, inputs: String) -> StageNodeBuilder {
        self.node.inputs = inputs;
        self
    }

    #[must_use]
    pub fn outputs(mut self, outputs: String) -> StageNodeBuilder {
        self.node.outputs = outputs;
        self
    }

    #[must_use]
    pub fn rules_in_context(mut self, rules_in_context: Vec<RuleInContext>) -> StageNodeBuilder {
        self.node.rules_in_context = rules_in_context;
        self
    }

    #[must_use]
    pub fn sensors_applicable(mut self, sensors_applicable: Vec<SensorRef>) -> StageNodeBuilder {
        self.node.sensors_applicable = sensors_applicable;
        self
    }

    #[must_use]
    pub fn build(self) -> StageNode {
        self.node
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn node(slug: &str, number: &str) -> StageNodeBuilder {
        StageNodeBuilder::new(
            StageSlug::parse(slug).unwrap(),
            StageNumber::parse(number).unwrap(),
            "Intent Capture".to_string(),
            PhaseId::Ideation,
            ExecutionKind::Always,
            StageMode::Inline,
        )
    }

    #[test]
    fn a_minimal_node_carries_identity_and_leaves_everything_else_empty() {
        let n = node("intent-capture", "1.1").build();
        assert_eq!(n.slug().as_str(), "intent-capture");
        assert_eq!(n.number().as_str(), "1.1");
        assert_eq!(n.phase(), PhaseId::Ideation);
        assert_eq!(n.mode(), StageMode::Inline);
        assert!(n.support_agents().is_empty());
        assert!(n.produces().is_empty());
        assert_eq!(n.reviewer(), None);
        assert_eq!(n.enabled(), None);
        // enabled のキー不在は「有効」
        assert!(n.is_enabled());
        assert!(!n.workspace_requires());
    }

    #[test]
    fn enabled_false_is_the_only_disabling_value() {
        assert!(node("s", "1.1").enabled(true).build().is_enabled());
        assert!(!node("s", "1.1").enabled(false).build().is_enabled());
    }

    #[test]
    fn rules_and_sensors_keep_their_object_shape_and_project_to_strings_on_demand() {
        let n = node("s", "1.1")
            .rules_in_context(vec![
                RuleInContext::new("aidlc/spaces/default/memory/org.md", RuleScope::Org),
                RuleInContext::new("aidlc/spaces/default/memory/ideation.md", RuleScope::Phase),
            ])
            .sensors_applicable(vec![SensorRef::new(
                "no-todo",
                ".claude/sensors/no-todo.md",
                Some("**/*.rs".to_string()),
            )])
            .build();
        // 格納形はオブジェクト配列 (scope / path / matches が生きている)
        assert_eq!(n.rules_in_context()[1].scope(), RuleScope::Phase);
        assert_eq!(n.sensors_applicable()[0].matches(), Some("**/*.rs"));
        // センサーの id と path は別物 — 射影 (`sensor_ids`) が潰すのは id 側だけ
        assert_eq!(n.sensors_applicable()[0].id(), "no-todo");
        assert_eq!(
            n.sensors_applicable()[0].path(),
            ".claude/sensors/no-todo.md"
        );
        // directive 射影は文字列配列
        assert_eq!(n.sensor_ids(), vec!["no-todo"]);
        assert_eq!(
            n.rule_paths(),
            vec![
                "aidlc/spaces/default/memory/org.md",
                "aidlc/spaces/default/memory/ideation.md"
            ]
        );
    }

    #[test]
    fn declares_scope_reads_the_transposition_source() {
        let n = node("s", "1.1")
            .scopes(vec!["feature".to_string(), "mvp".to_string()])
            .build();
        assert!(n.declares_scope("feature"));
        assert!(!n.declares_scope("bugfix"));
        assert!(!n.declares_scope("Feature"));
    }

    #[test]
    fn rule_scope_and_conditional_on_are_closed_sets() {
        for r in RuleScope::ALL {
            assert_eq!(RuleScope::parse(r.as_str()).unwrap(), r);
        }
        // 閉集合外は生値を逐語で持ち帰る (`space` は別語彙であって rule scope ではない)
        let unknown_scope = RuleScope::parse("space").unwrap_err();
        assert_eq!(unknown_scope.as_str(), "space");
        assert_eq!(unknown_scope, UnknownRuleScope::new("space"));
        for b in BrownfieldGreenfield::ALL {
            assert_eq!(BrownfieldGreenfield::parse(b.as_str()).unwrap(), b);
        }
        let unknown_field = BrownfieldGreenfield::parse("bluefield").unwrap_err();
        assert_eq!(unknown_field.as_str(), "bluefield");
        assert_eq!(unknown_field, UnknownBrownfieldGreenfield::new("bluefield"));
    }

    #[test]
    fn consumes_keeps_required_and_conditional_on_separate() {
        let n = node("s", "1.1")
            .consumes(vec![
                ConsumeDecl::new("requirements", true, None),
                ConsumeDecl::new(
                    "codebase-analysis",
                    false,
                    Some(BrownfieldGreenfield::Brownfield),
                ),
            ])
            .build();
        assert_eq!(n.consumes()[0].artifact(), "requirements");
        assert!(n.consumes()[0].required());
        assert_eq!(n.consumes()[0].conditional_on(), None);
        assert_eq!(n.consumes()[1].artifact(), "codebase-analysis");
        assert!(!n.consumes()[1].required());
        assert_eq!(
            n.consumes()[1].conditional_on(),
            Some(BrownfieldGreenfield::Brownfield)
        );
    }

    proptest! {
        /// builder の setter は他フィールドを汚染しない (28 フィールドの独立性)。
        #[test]
        fn setters_do_not_bleed_into_other_fields(
            cond in "[a-z ]{0,20}",
            reviewer in "[a-z-]{1,10}",
            iterations in 1u32..10,
            scopes in proptest::collection::vec("[a-z]{1,8}", 0..4),
        ) {
            let n = node("s", "1.1")
                .condition(cond.clone())
                .reviewer(reviewer.clone())
                .reviewer_max_iterations(iterations)
                .scopes(scopes.clone())
                .build();
            prop_assert_eq!(n.condition(), cond.as_str());
            prop_assert_eq!(n.reviewer(), Some(reviewer.as_str()));
            prop_assert_eq!(n.reviewer_max_iterations(), Some(iterations));
            prop_assert_eq!(n.scopes(), scopes.as_slice());
            // 触っていないフィールドは既定値のまま
            prop_assert_eq!(n.inputs(), "");
            prop_assert_eq!(n.outputs(), "");
            prop_assert_eq!(n.review_class(), None);
            prop_assert!(n.sensors_applicable().is_empty());
        }

        /// directive 射影は常に格納形と同じ長さ・同じ順序 (潰しても順序は失わない)。
        #[test]
        fn projections_preserve_length_and_order(
            ids in proptest::collection::vec("[a-z]{1,6}", 0..6),
            paths in proptest::collection::vec("[a-z/]{1,10}", 0..6),
        ) {
            let n = node("s", "1.1")
                .sensors_applicable(
                    ids.iter()
                        .map(|id| {
                            SensorRef::new(id.clone(), format!(".claude/sensors/{id}.md"), None)
                        })
                        .collect(),
                )
                .rules_in_context(
                    paths
                        .iter()
                        .map(|p| RuleInContext::new(p.clone(), RuleScope::Project))
                        .collect(),
                )
                .build();
            prop_assert_eq!(n.sensor_ids(), ids.iter().map(String::as_str).collect::<Vec<_>>());
            prop_assert_eq!(n.rule_paths(), paths.iter().map(String::as_str).collect::<Vec<_>>());
        }
    }
}
