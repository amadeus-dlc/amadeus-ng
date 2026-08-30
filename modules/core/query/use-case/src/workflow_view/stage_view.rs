//! `StageView` — コンパイル済み `stage-graph.json` の 1 要素 (`FIELD_ORDER` 28 フィールド) の
//! 型付きビュー (12 §2.2)。
//!
//! 形の観測可能契約 (12 §6.1-3/4/5):
//! - `rules_in_context` は**オブジェクト配列** `{path, scope}`、`sensors_applicable` も
//!   **オブジェクト配列** `{id, path, matches?}`。**文字列配列に潰さない**
//!   (directive 側の `string[]` は射影であって格納形ではない — [`StageView::sensor_ids`] /
//!   [`StageView::rule_paths`] がその射影を明示的に提供する)。
//! - `inputs` / `outputs` は**文字列** (配列にしない)。機械可読な出力は `produces`。
//! - `number` は文字列 `"P.I"` ([`StageNumberView`] が逐語保持)。
//!
//! 相互フィールド不変条件 (`mode ∈ {pipeline, mob} ⇒ support_agents` 非空など) は upstream
//! では **compile 時**に検証され、読取経路では再検査されない (12 §4.3)。本ビューも同様に
//! 再検査しない — 「グラフ全体が読めない」と「1 ノードが使えない」の観測差を作らない。

use std::collections::BTreeMap;

use super::consume_decl_view::ConsumeDeclView;
use super::execution_kind_view::ExecutionKindView;
use super::phase_view::PhaseView;
use super::review_class_view::ReviewClassView;
use super::rule_in_context_view::RuleInContextView;
use super::sensor_ref_view::SensorRefView;
use super::stage_mode_view::StageModeView;
use super::stage_number_view::StageNumberView;
use super::stage_slug_view::StageSlugView;

/// コンパイル済みグラフの 1 ノード。フィールドは `FIELD_ORDER` の 28 エントリに 1:1 対応する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageView {
    slug: StageSlugView,
    number: StageNumberView,
    name: String,
    phase: PhaseView,
    execution: ExecutionKindView,
    condition: String,
    lead_agent: String,
    support_agents: Vec<String>,
    mode: StageModeView,
    for_each: Option<String>,
    workspace_requires: bool,
    produces: Vec<String>,
    optional_produces: Vec<String>,
    produces_kinds: BTreeMap<String, Vec<String>>,
    consumes: Vec<ConsumeDeclView>,
    requires_stage: Vec<StageSlugView>,
    sensors: Vec<String>,
    scopes: Vec<String>,
    reviewer: Option<String>,
    reviewer_max_iterations: Option<u32>,
    review_class: Option<ReviewClassView>,
    summary_confirmation: Option<String>,
    plugin: Option<String>,
    enabled: Option<bool>,
    inputs: String,
    outputs: String,
    rules_in_context: Vec<RuleInContextView>,
    sensors_applicable: Vec<SensorRefView>,
}

/// [`StageView`] の唯一の構成経路。
///
/// `new()` の 6 引数は全ノードに必ず存在する型付きコアで、残り 22 フィールドは既定値
/// (空コレクション / 空文字列 / `None` / `false`) から `with_*` で積み上げる。
/// `mut self` を取って `Self` を返すので setter ではなくファクトリメソッドである
/// (`coding-rules/factory-naming.md`)。
#[derive(Debug, Clone)]
pub struct StageViewBuilder {
    node: StageView,
}

impl StageView {
    // ---- identity ----

    /// グラフ内で一意な識別子。ステージファイル名の stem と一致する。
    #[must_use]
    pub const fn slug(&self) -> &StageSlugView {
        &self.slug
    }

    /// エンジンが割り当てた `"<phaseIndex>.<seq>"`。比較は数値順のみで行う。
    #[must_use]
    pub const fn number(&self) -> &StageNumberView {
        &self.number
    }

    /// 表示名。著者の `name:` が無ければ slug の title case が入る。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 所属フェーズ。`rules_in_context` の phase 層が載る条件でもある。
    #[must_use]
    pub const fn phase(&self) -> PhaseView {
        self.phase
    }

    // ---- 適用可否 ----

    /// ステージ著者側の適用可否。プラン所属 (EXECUTE / SKIP) とは直交する。
    #[must_use]
    pub const fn execution(&self) -> ExecutionKindView {
        self.execution
    }

    /// 自由記述の適用ルール (人間・LLM 向け。機械可読ではない)。
    #[must_use]
    pub fn condition(&self) -> &str {
        &self.condition
    }

    // ---- ルーティング ----

    /// 主エージェントの slug。予約疑似エージェント `orchestrator` を取りうる。
    #[must_use]
    pub fn lead_agent(&self) -> &str {
        &self.lead_agent
    }

    /// 補助エージェントの slug 列。`mode ∈ {pipeline, mob}` では非空 (compile 時に検証済み)。
    #[must_use]
    pub fn support_agents(&self) -> &[String] {
        &self.support_agents
    }

    /// 実行トポロジ。`agent-team` は予約値であり、既定経路へフォールスルーさせてはならない。
    #[must_use]
    pub const fn mode(&self) -> StageModeView {
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

    /// 入力成果物の宣言列。欠損時の態度は各エントリの `required` が決める。
    #[must_use]
    pub fn consumes(&self) -> &[ConsumeDeclView] {
        &self.consumes
    }

    /// 依存ステージ。compile 時に dedup 済みで、数値順で自分より前にある。
    #[must_use]
    pub fn requires_stage(&self) -> &[StageSlugView] {
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

    /// レビュアーエージェントの slug。`None` はレビュアー宣言が無いステージ。
    #[must_use]
    pub fn reviewer(&self) -> Option<&str> {
        self.reviewer.as_deref()
    }

    /// 正整数。`reviewer` 宣言が前提。
    #[must_use]
    pub const fn reviewer_max_iterations(&self) -> Option<u32> {
        self.reviewer_max_iterations
    }

    /// レビューの重量宣言。`reviewer` 宣言が前提。
    #[must_use]
    pub const fn review_class(&self) -> Option<ReviewClassView> {
        self.review_class
    }

    // ---- その他の宣言 ----

    /// 観測値は `"required"` のみ。値域は未確定 (12 §7) のため文字列で保持する。
    #[must_use]
    pub fn summary_confirmation(&self) -> Option<&str> {
        self.summary_confirmation.as_deref()
    }

    /// 所有プラグイン名 (frontmatter からの逐語コピー)。
    #[must_use]
    pub fn plugin(&self) -> Option<&str> {
        self.plugin.as_deref()
    }

    /// `enabled` の生値。**`None` は「キー不在」= 有効**を意味する (12 §6.3-5)。
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

    /// compile 時に解決済みのルール行。長さは 3 (org+team+project) か 4 (+ 該当 phase)。
    #[must_use]
    pub fn rules_in_context(&self) -> &[RuleInContextView] {
        &self.rules_in_context
    }

    /// compile 時に確定したセンサー適用宣言。fire 時に manifest を再オープンしない。
    #[must_use]
    pub fn sensors_applicable(&self) -> &[SensorRefView] {
        &self.sensors_applicable
    }

    // ---- directive 射影 (12 §2.3 — 格納形と混同しないための明示 API) ----

    /// directive 上の `sensors_applicable` は **id の `string[]`** に潰れる。
    #[must_use]
    pub fn sensor_ids(&self) -> Vec<&str> {
        self.sensors_applicable
            .iter()
            .map(SensorRefView::id)
            .collect()
    }

    /// directive 上の `rules_in_context` は**パスの `string[]`** に潰れる (順序は宣言順)。
    #[must_use]
    pub fn rule_paths(&self) -> Vec<&str> {
        self.rules_in_context
            .iter()
            .map(RuleInContextView::path)
            .collect()
    }
}

impl StageViewBuilder {
    /// 全ノードに必ず存在する型付きコア 6 値でビルダを起こす。
    ///
    /// 残り 22 フィールドは既定値 (空コレクション / 空文字列 / `None` / `false`) から始まる。
    /// [`StageView`] の構造体リテラルが現れるのはここ 1 か所だけである
    /// (`coding-rules/factory-naming.md`)。
    #[must_use]
    pub const fn new(
        slug: StageSlugView,
        number: StageNumberView,
        name: String,
        phase: PhaseView,
        execution: ExecutionKindView,
        mode: StageModeView,
    ) -> StageViewBuilder {
        StageViewBuilder {
            node: StageView {
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

    /// 自由記述の適用ルールを載せる。未指定は空文字列。
    #[must_use]
    pub fn with_condition(mut self, condition: String) -> StageViewBuilder {
        self.node.condition = condition;
        self
    }

    /// 主エージェント slug を載せる。未指定は空文字列。
    #[must_use]
    pub fn with_lead_agent(mut self, lead_agent: String) -> StageViewBuilder {
        self.node.lead_agent = lead_agent;
        self
    }

    /// 補助エージェント slug 列を載せる。未指定は空。
    #[must_use]
    pub fn with_support_agents(mut self, support_agents: Vec<String>) -> StageViewBuilder {
        self.node.support_agents = support_agents;
        self
    }

    /// 反復軸を載せる (観測値は `"unit-of-work"` のみ)。呼ばなければ `None` = 反復しない。
    #[must_use]
    pub fn with_for_each(mut self, for_each: String) -> StageViewBuilder {
        self.node.for_each = Some(for_each);
        self
    }

    /// ワークスペース実体への書込要求を載せる。未指定は `false`。
    #[must_use]
    pub const fn with_workspace_requires(mut self, workspace_requires: bool) -> StageViewBuilder {
        self.node.workspace_requires = workspace_requires;
        self
    }

    /// 成果物の語彙名列を載せる。未指定は空。
    #[must_use]
    pub fn with_produces(mut self, produces: Vec<String>) -> StageViewBuilder {
        self.node.produces = produces;
        self
    }

    /// 条件付き成果物の語彙名列を載せる。未指定は空。
    #[must_use]
    pub fn with_optional_produces(mut self, optional_produces: Vec<String>) -> StageViewBuilder {
        self.node.optional_produces = optional_produces;
        self
    }

    /// 成果物 → 適用 unit kind の写像を載せる。未指定は空 = 全成果物が全 kind に適用される。
    #[must_use]
    pub fn with_produces_kinds(
        mut self,
        produces_kinds: BTreeMap<String, Vec<String>>,
    ) -> StageViewBuilder {
        self.node.produces_kinds = produces_kinds;
        self
    }

    /// 入力成果物の宣言列を載せる。未指定は空。
    #[must_use]
    pub fn with_consumes(mut self, consumes: Vec<ConsumeDeclView>) -> StageViewBuilder {
        self.node.consumes = consumes;
        self
    }

    /// 依存ステージ slug 列を載せる。未指定は空。dedup とエッジ順序の検査は compile 側。
    #[must_use]
    pub fn with_requires_stage(mut self, requires_stage: Vec<StageSlugView>) -> StageViewBuilder {
        self.node.requires_stage = requires_stage;
        self
    }

    /// 著者が pull import したセンサー id 列を載せる。未指定は空。
    #[must_use]
    pub fn with_sensors(mut self, sensors: Vec<String>) -> StageViewBuilder {
        self.node.sensors = sensors;
        self
    }

    /// このステージを EXECUTE にするスコープ名を載せる。未指定は空。
    #[must_use]
    pub fn with_scopes(mut self, scopes: Vec<String>) -> StageViewBuilder {
        self.node.scopes = scopes;
        self
    }

    /// レビュアー slug を載せる。呼ばなければ `None` = レビュアー無し。
    #[must_use]
    pub fn with_reviewer(mut self, reviewer: String) -> StageViewBuilder {
        self.node.reviewer = Some(reviewer);
        self
    }

    /// レビュー反復の上限を載せる。`reviewer` と対で宣言されるのが前提。
    #[must_use]
    pub const fn with_reviewer_max_iterations(mut self, iterations: u32) -> StageViewBuilder {
        self.node.reviewer_max_iterations = Some(iterations);
        self
    }

    /// レビューの重量宣言を載せる。`reviewer` と対で宣言されるのが前提。
    #[must_use]
    pub const fn with_review_class(mut self, review_class: ReviewClassView) -> StageViewBuilder {
        self.node.review_class = Some(review_class);
        self
    }

    /// `summary_confirmation` を載せる。観測値は `"required"` のみ。
    #[must_use]
    pub fn with_summary_confirmation(mut self, summary_confirmation: String) -> StageViewBuilder {
        self.node.summary_confirmation = Some(summary_confirmation);
        self
    }

    /// 所有プラグイン名を載せる。frontmatter からの逐語コピーであり、ここで加工しない。
    #[must_use]
    pub fn with_plugin(mut self, plugin: String) -> StageViewBuilder {
        self.node.plugin = Some(plugin);
        self
    }

    /// `enabled` を明示的に立てる。呼ばなければ `None` (= 有効) のまま。
    #[must_use]
    pub const fn with_enabled(mut self, enabled: bool) -> StageViewBuilder {
        self.node.enabled = Some(enabled);
        self
    }

    /// 散文の `inputs` を載せる。未指定は空文字列。配列に分解しない。
    #[must_use]
    pub fn with_inputs(mut self, inputs: String) -> StageViewBuilder {
        self.node.inputs = inputs;
        self
    }

    /// 散文の `outputs` を載せる。未指定は空文字列。機械可読な出力は `produces` 側。
    #[must_use]
    pub fn with_outputs(mut self, outputs: String) -> StageViewBuilder {
        self.node.outputs = outputs;
        self
    }

    /// compile が解決したルール行を載せる。著者が書ける値ではない。
    #[must_use]
    pub fn with_rules_in_context(
        mut self,
        rules_in_context: Vec<RuleInContextView>,
    ) -> StageViewBuilder {
        self.node.rules_in_context = rules_in_context;
        self
    }

    /// compile が確定したセンサー適用宣言を載せる。著者が書ける値ではない。
    #[must_use]
    pub fn with_sensors_applicable(
        mut self,
        sensors_applicable: Vec<SensorRefView>,
    ) -> StageViewBuilder {
        self.node.sensors_applicable = sensors_applicable;
        self
    }

    /// 積み上げを終える。相互フィールド不変条件はここで再検査しない (compile 側の責務)。
    #[must_use]
    pub fn build(self) -> StageView {
        self.node
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::indexing_slicing)]

    use super::*;
    use crate::workflow_view::RuleScopeView;

    fn node(slug: &str, number: &str) -> StageViewBuilder {
        StageViewBuilder::new(
            StageSlugView::parse(slug).unwrap(),
            StageNumberView::parse(number).unwrap(),
            "Intent Capture".to_string(),
            PhaseView::Ideation,
            ExecutionKindView::Always,
            StageModeView::Inline,
        )
    }

    #[test]
    fn a_minimal_node_carries_identity_and_leaves_everything_else_empty() {
        let n = node("intent-capture", "1.1").build();
        assert_eq!(n.slug().as_str(), "intent-capture");
        assert_eq!(n.number().as_str(), "1.1");
        assert_eq!(n.name(), "Intent Capture");
        assert_eq!(n.phase(), PhaseView::Ideation);
        assert_eq!(n.execution(), ExecutionKindView::Always);
        assert_eq!(n.mode(), StageModeView::Inline);
        assert_eq!(n.condition(), "");
        assert_eq!(n.lead_agent(), "");
        assert!(n.support_agents().is_empty());
        assert_eq!(n.for_each(), None);
        assert!(!n.workspace_requires());
        assert!(n.produces().is_empty());
        assert!(n.optional_produces().is_empty());
        assert!(n.produces_kinds().is_empty());
        assert!(n.consumes().is_empty());
        assert!(n.requires_stage().is_empty());
        assert!(n.sensors().is_empty());
        assert!(n.scopes().is_empty());
        assert_eq!(n.reviewer(), None);
        assert_eq!(n.reviewer_max_iterations(), None);
        assert_eq!(n.review_class(), None);
        assert_eq!(n.summary_confirmation(), None);
        assert_eq!(n.plugin(), None);
        assert_eq!(n.enabled(), None);
        assert_eq!(n.inputs(), "");
        assert_eq!(n.outputs(), "");
    }

    #[test]
    fn enabled_false_is_the_only_disabling_value() {
        assert!(node("s", "1.1").build().is_enabled());
        assert!(node("s", "1.1").with_enabled(true).build().is_enabled());
        assert!(!node("s", "1.1").with_enabled(false).build().is_enabled());
    }

    #[test]
    fn rules_and_sensors_keep_their_object_shape_and_project_to_strings_on_demand() {
        let n = node("s", "1.1")
            .with_rules_in_context(vec![
                RuleInContextView::new("aidlc/spaces/default/memory/org.md", RuleScopeView::Org),
                RuleInContextView::new(
                    "aidlc/spaces/default/memory/ideation.md",
                    RuleScopeView::Phase,
                ),
            ])
            .with_sensors_applicable(vec![SensorRefView::new(
                "no-todo",
                ".claude/sensors/no-todo.md",
                Some("**/*.rs".to_string()),
            )])
            .build();
        assert_eq!(n.rules_in_context()[1].scope(), RuleScopeView::Phase);
        assert_eq!(n.sensors_applicable()[0].matches(), Some("**/*.rs"));
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
            .with_scopes(vec!["feature".to_string(), "mvp".to_string()])
            .build();
        assert!(n.declares_scope("feature"));
        assert!(!n.declares_scope("bugfix"));
        assert!(!n.declares_scope("Feature"));
    }

    #[test]
    fn the_builder_setters_do_not_bleed_into_other_fields() {
        let n = node("s", "1.1")
            .with_condition("when units exist".to_string())
            .with_reviewer("adversarial-reviewer".to_string())
            .with_reviewer_max_iterations(3)
            .with_review_class(ReviewClassView::Adversarial)
            .build();
        assert_eq!(n.condition(), "when units exist");
        assert_eq!(n.reviewer(), Some("adversarial-reviewer"));
        assert_eq!(n.reviewer_max_iterations(), Some(3));
        assert_eq!(n.review_class(), Some(ReviewClassView::Adversarial));
        assert_eq!(n.inputs(), "");
        assert_eq!(n.outputs(), "");
        assert!(n.sensors_applicable().is_empty());
    }
}
