//! `StageNodeBuilder` — `StageNode` の唯一の構成経路。
//!
//! `StageNode` の構造体リテラルを書く唯一の場所なので、兄弟ではなく本型のサブツリーに置く
//! (子モジュールは親の private フィールドを見られるので、可視性を広げずに 1 ファイル
//! 1 公開型にできる — `coding-rules/field-visibility.md` / `factory-naming.md`)。

use std::collections::BTreeMap;

use super::StageNode;
use crate::workflow_definition::{
    ConsumeDecl, ExecutionKind, PhaseId, ReviewClass, RuleInContext, SensorRef, StageMode,
    StageNumber, StageSlug,
};

/// `StageNode` の唯一の構成経路。
///
/// `new()` の 6 引数は全ノードに必ず存在する型付きコアで、残りは既定値
/// (空コレクション / 空文字列 / `None` / `false`) から setter で積み上げる。
#[derive(Debug, Clone)]
pub struct StageNodeBuilder {
    node: StageNode,
}

impl StageNodeBuilder {
    /// 全ノードに必ず存在する型付きコア 6 値でビルダを起こす。残り 22 フィールドは
    /// 既定値 (空コレクション / 空文字列 / `None` / `false`) から始まる。
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

    /// 自由記述の適用ルールを載せる。未指定は空文字列。
    #[must_use]
    pub fn condition(mut self, condition: String) -> StageNodeBuilder {
        self.node.condition = condition;
        self
    }

    /// 主エージェント slug を載せる。未指定は空文字列。
    #[must_use]
    pub fn lead_agent(mut self, lead_agent: String) -> StageNodeBuilder {
        self.node.lead_agent = lead_agent;
        self
    }

    /// 補助エージェント slug 列を載せる。未指定は空。
    #[must_use]
    pub fn support_agents(mut self, support_agents: Vec<String>) -> StageNodeBuilder {
        self.node.support_agents = support_agents;
        self
    }

    /// 反復軸を載せる (観測値は `"unit-of-work"` のみ)。呼ばなければ `None` = 反復しない。
    #[must_use]
    pub fn for_each(mut self, for_each: String) -> StageNodeBuilder {
        self.node.for_each = Some(for_each);
        self
    }

    /// ワークスペース実体への書込要求を載せる。未指定は `false`。
    #[must_use]
    pub const fn workspace_requires(mut self, workspace_requires: bool) -> StageNodeBuilder {
        self.node.workspace_requires = workspace_requires;
        self
    }

    /// 成果物の語彙名列を載せる。未指定は空。
    #[must_use]
    pub fn produces(mut self, produces: Vec<String>) -> StageNodeBuilder {
        self.node.produces = produces;
        self
    }

    /// 条件付き成果物の語彙名列を載せる。未指定は空。
    #[must_use]
    pub fn optional_produces(mut self, optional_produces: Vec<String>) -> StageNodeBuilder {
        self.node.optional_produces = optional_produces;
        self
    }

    /// 成果物 → 適用 unit kind の写像を載せる。未指定は空 = 全成果物が全 kind に適用される。
    #[must_use]
    pub fn produces_kinds(
        mut self,
        produces_kinds: BTreeMap<String, Vec<String>>,
    ) -> StageNodeBuilder {
        self.node.produces_kinds = produces_kinds;
        self
    }

    /// 入力成果物の宣言列を載せる。未指定は空。
    #[must_use]
    pub fn consumes(mut self, consumes: Vec<ConsumeDecl>) -> StageNodeBuilder {
        self.node.consumes = consumes;
        self
    }

    /// 依存ステージ slug 列を載せる。未指定は空。dedup とエッジ順序の検査は compile 側。
    #[must_use]
    pub fn requires_stage(mut self, requires_stage: Vec<StageSlug>) -> StageNodeBuilder {
        self.node.requires_stage = requires_stage;
        self
    }

    /// 著者が pull import したセンサー id 列を載せる。未指定は空。
    #[must_use]
    pub fn sensors(mut self, sensors: Vec<String>) -> StageNodeBuilder {
        self.node.sensors = sensors;
        self
    }

    /// このステージを EXECUTE にするスコープ名を載せる。未指定は空。
    #[must_use]
    pub fn scopes(mut self, scopes: Vec<String>) -> StageNodeBuilder {
        self.node.scopes = scopes;
        self
    }

    /// レビュアー slug を載せる。呼ばなければ `None` = レビュアー無し。
    #[must_use]
    pub fn reviewer(mut self, reviewer: String) -> StageNodeBuilder {
        self.node.reviewer = Some(reviewer);
        self
    }

    /// レビュー反復の上限を載せる。`reviewer` と対で宣言されるのが前提。
    #[must_use]
    pub const fn reviewer_max_iterations(mut self, iterations: u32) -> StageNodeBuilder {
        self.node.reviewer_max_iterations = Some(iterations);
        self
    }

    /// レビューの重量宣言を載せる。`reviewer` と対で宣言されるのが前提。
    #[must_use]
    pub const fn review_class(mut self, review_class: ReviewClass) -> StageNodeBuilder {
        self.node.review_class = Some(review_class);
        self
    }

    /// `summary_confirmation` を載せる。観測値は `"required"` のみ。
    #[must_use]
    pub fn summary_confirmation(mut self, summary_confirmation: String) -> StageNodeBuilder {
        self.node.summary_confirmation = Some(summary_confirmation);
        self
    }

    /// 所有プラグイン名を載せる。frontmatter からの逐語コピーであり、ここで加工しない。
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

    /// 散文の `inputs` を載せる。未指定は空文字列。配列に分解しない。
    #[must_use]
    pub fn inputs(mut self, inputs: String) -> StageNodeBuilder {
        self.node.inputs = inputs;
        self
    }

    /// 散文の `outputs` を載せる。未指定は空文字列。機械可読な出力は `produces` 側。
    #[must_use]
    pub fn outputs(mut self, outputs: String) -> StageNodeBuilder {
        self.node.outputs = outputs;
        self
    }

    /// compile が解決したルール行を載せる。著者が書ける値ではない。
    #[must_use]
    pub fn rules_in_context(mut self, rules_in_context: Vec<RuleInContext>) -> StageNodeBuilder {
        self.node.rules_in_context = rules_in_context;
        self
    }

    /// compile が確定したセンサー適用宣言を載せる。著者が書ける値ではない。
    #[must_use]
    pub fn sensors_applicable(mut self, sensors_applicable: Vec<SensorRef>) -> StageNodeBuilder {
        self.node.sensors_applicable = sensors_applicable;
        self
    }

    /// 積み上げを終える。相互フィールド不変条件はここで再検査しない (compile 側の責務)。
    #[must_use]
    pub fn build(self) -> StageNode {
        self.node
    }
}
