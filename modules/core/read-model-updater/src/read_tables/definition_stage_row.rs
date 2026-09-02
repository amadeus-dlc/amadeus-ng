//! `DefinitionStageRow` — `read_definition_stage` の 1 行 (グラフのノード 1 件を全列で写す)。

use core_command_domain::orchestration::StageKey;
use core_command_domain::workflow_definition::{ReviewClass, StageNode, WorkflowDefinitionId};

use super::json_column;

/// `read_definition_stage` の 1 行。主キーは (`definition_id`, `stage_slug`)。
///
/// [`StageNode`] の 29 アクセサを 1 行に平らに写す。配列・構造は `ContractCompact` の
/// 1 行 JSON にする — 読取コマンドが 1 回の引当で全属性を得るための非正規化である
/// (裁定 §10-1)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionStageRow {
    definition_id: String,
    stage_slug: String,
    position: usize,
    number: String,
    name: String,
    phase: String,
    execution: String,
    condition: String,
    lead_agent: String,
    support_agents: String,
    mode: String,
    for_each: Option<String>,
    workspace_requires: bool,
    produces: String,
    optional_produces: String,
    produces_kinds: String,
    consumes: String,
    requires_stage: String,
    sensors: String,
    scopes: String,
    reviewer: Option<String>,
    reviewer_max_iterations: Option<u32>,
    review_class: Option<String>,
    summary_confirmation: Option<String>,
    plugin: Option<String>,
    enabled: Option<bool>,
    gated: bool,
    inputs: String,
    outputs: String,
    rules_in_context: String,
    sensors_applicable: String,
}

impl DefinitionStageRow {
    /// グラフのノード 1 件を 1 行へ写す (**この型の唯一の構築経路**)。
    ///
    /// `position` は文書順 (グラフの並びそのもの)。`gated` はドメインの述語
    /// [`StageKey::is_gated`] に問う — 「initialization だけが非ゲート」という規則を
    /// ここで書き直さない。
    #[must_use]
    pub fn of(
        definition_id: &WorkflowDefinitionId,
        position: usize,
        node: &StageNode,
    ) -> DefinitionStageRow {
        let key = StageKey::new(node.slug().clone(), node.phase());
        DefinitionStageRow {
            definition_id: definition_id.as_str().to_string(),
            stage_slug: node.slug().as_str().to_string(),
            position,
            number: node.number().as_str().to_string(),
            name: node.name().to_string(),
            phase: node.phase().as_str().to_string(),
            execution: node.execution().as_str().to_string(),
            condition: node.condition().to_string(),
            lead_agent: node.lead_agent().to_string(),
            support_agents: json_column::strings(node.support_agents()),
            mode: node.mode().as_str().to_string(),
            for_each: node.for_each().map(str::to_string),
            workspace_requires: node.workspace_requires(),
            produces: json_column::strings(node.produces()),
            optional_produces: json_column::strings(node.optional_produces()),
            produces_kinds: json_column::produces_kinds(node.produces_kinds()),
            consumes: json_column::consumes(node.consumes()),
            requires_stage: json_column::slugs(node.requires_stage()),
            sensors: json_column::strings(node.sensors()),
            scopes: json_column::strings(node.scopes()),
            reviewer: node.reviewer().map(str::to_string),
            reviewer_max_iterations: node.reviewer_max_iterations(),
            review_class: node
                .review_class()
                .map(ReviewClass::as_str)
                .map(str::to_string),
            summary_confirmation: node.summary_confirmation().map(str::to_string),
            plugin: node.plugin().map(str::to_string),
            enabled: node.enabled(),
            gated: key.is_gated(),
            inputs: node.inputs().to_string(),
            outputs: node.outputs().to_string(),
            rules_in_context: json_column::rules_in_context(node.rules_in_context()),
            sensors_applicable: json_column::sensors_applicable(node.sensors_applicable()),
        }
    }

    /// 定義の系譜 ID。
    #[must_use]
    pub fn definition_id(&self) -> &str {
        &self.definition_id
    }

    /// ステージの slug。
    #[must_use]
    pub fn stage_slug(&self) -> &str {
        &self.stage_slug
    }

    /// 文書順の位置 (0 始まり)。
    #[must_use]
    pub const fn position(&self) -> usize {
        self.position
    }

    /// `<phaseIndex>.<seq>` のステージ番号。
    #[must_use]
    pub fn number(&self) -> &str {
        &self.number
    }

    /// 表示名。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// フェーズの綴り (`PhaseId::as_str`)。
    #[must_use]
    pub fn phase(&self) -> &str {
        &self.phase
    }

    /// 実行種別の綴り (`ExecutionKind::as_str`)。
    #[must_use]
    pub fn execution(&self) -> &str {
        &self.execution
    }

    /// 条件付き実行の条件式 (無条件なら空文字)。
    #[must_use]
    pub fn condition(&self) -> &str {
        &self.condition
    }

    /// 主担当エージェント。
    #[must_use]
    pub fn lead_agent(&self) -> &str {
        &self.lead_agent
    }

    /// 支援エージェントの 1 行 JSON 配列。
    #[must_use]
    pub fn support_agents(&self) -> &str {
        &self.support_agents
    }

    /// 実行様式の綴り (`StageMode::as_str`)。
    #[must_use]
    pub fn mode(&self) -> &str {
        &self.mode
    }

    /// 反復軸 (無ければ NULL)。
    #[must_use]
    pub fn for_each(&self) -> Option<&str> {
        self.for_each.as_deref()
    }

    /// ワークスペースを要求するか。
    #[must_use]
    pub const fn workspace_requires(&self) -> bool {
        self.workspace_requires
    }

    /// 必須成果物の 1 行 JSON 配列。
    #[must_use]
    pub fn produces(&self) -> &str {
        &self.produces
    }

    /// 任意成果物の 1 行 JSON 配列。
    #[must_use]
    pub fn optional_produces(&self) -> &str {
        &self.optional_produces
    }

    /// 成果物と種別の対の 1 行 JSON 配列。
    #[must_use]
    pub fn produces_kinds(&self) -> &str {
        &self.produces_kinds
    }

    /// 上流成果物の宣言の 1 行 JSON 配列。
    #[must_use]
    pub fn consumes(&self) -> &str {
        &self.consumes
    }

    /// 先行必須ステージの 1 行 JSON 配列。
    #[must_use]
    pub fn requires_stage(&self) -> &str {
        &self.requires_stage
    }

    /// 宣言センサー ID の 1 行 JSON 配列。
    #[must_use]
    pub fn sensors(&self) -> &str {
        &self.sensors
    }

    /// このノードが宣言するスコープ名の 1 行 JSON 配列。
    #[must_use]
    pub fn scopes(&self) -> &str {
        &self.scopes
    }

    /// レビュアーのエージェント名 (無ければ NULL)。
    #[must_use]
    pub fn reviewer(&self) -> Option<&str> {
        self.reviewer.as_deref()
    }

    /// レビュー往復の上限 (無ければ NULL)。
    #[must_use]
    pub const fn reviewer_max_iterations(&self) -> Option<u32> {
        self.reviewer_max_iterations
    }

    /// レビュー階級の綴り (無ければ NULL)。
    #[must_use]
    pub fn review_class(&self) -> Option<&str> {
        self.review_class.as_deref()
    }

    /// 要約確認の宣言 (無ければ NULL)。
    #[must_use]
    pub fn summary_confirmation(&self) -> Option<&str> {
        self.summary_confirmation.as_deref()
    }

    /// このノードを供給したプラグイン名 (無ければ NULL)。
    #[must_use]
    pub fn plugin(&self) -> Option<&str> {
        self.plugin.as_deref()
    }

    /// プラグイン選択で有効化されているか (宣言が無ければ NULL)。
    #[must_use]
    pub const fn enabled(&self) -> Option<bool> {
        self.enabled
    }

    /// 承認ゲート付きか (`StageKey::is_gated` の答え)。
    #[must_use]
    pub const fn gated(&self) -> bool {
        self.gated
    }

    /// 入力の散文。
    #[must_use]
    pub fn inputs(&self) -> &str {
        &self.inputs
    }

    /// 出力の散文。
    #[must_use]
    pub fn outputs(&self) -> &str {
        &self.outputs
    }

    /// 解決済みルール行の 1 行 JSON 配列。
    #[must_use]
    pub fn rules_in_context(&self) -> &str {
        &self.rules_in_context
    }

    /// 適用センサー宣言の 1 行 JSON 配列。
    #[must_use]
    pub fn sensors_applicable(&self) -> &str {
        &self.sensors_applicable
    }
}
