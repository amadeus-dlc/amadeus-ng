//! `StageViewBuilder` — [`StageView`] の唯一の構成経路。
//!
//! `new()` の 6 引数は全ノードに必ず存在する型付きコアで、残り 22 フィールドは既定値
//! (空コレクション / 空文字列 / `None` / `false`) から `with_*` で積み上げる。
//! `mut self` を取って `Self` を返すので setter ではなくファクトリメソッドである
//! (`coding-rules/factory-naming.md`)。
//!
//! 対象型の**子モジュール**に置くのは、private フィールドが「定義モジュールとその子孫」まで
//! 見えるからである — 兄弟ファイルへ出すと、28 フィールドを位置引数で受け渡す基本
//! コンストラクタを別に立てることになり、取り違えを型で防げなくなる。

use std::collections::BTreeMap;

use super::super::consume_decl_view::ConsumeDeclView;
use super::super::execution_kind_view::ExecutionKindView;
use super::super::phase_view::PhaseView;
use super::super::review_class_view::ReviewClassView;
use super::super::rule_in_context_view::RuleInContextView;
use super::super::sensor_ref_view::SensorRefView;
use super::super::stage_mode_view::StageModeView;
use super::super::stage_number_view::StageNumberView;
use super::super::stage_slug_view::StageSlugView;
use super::StageView;

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
