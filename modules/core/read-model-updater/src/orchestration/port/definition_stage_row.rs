//! `DefinitionStageRow` — `read_definition_stage` の 1 行 (グラフのノード 1 件を全列で写す)。

/// `read_definition_stage` の 1 行。主キーは 1 列 `id` (自然キー
/// (`definition_id`, `stage_slug`) から導いた代理キー)。`definition_id` は
/// `read_definition.id` を指す FK である。
///
/// [`StageNode`] の 29 アクセサを 1 行に平らに写す。配列・構造は `ContractCompact` の
/// 1 行 JSON にする — 読取コマンドが 1 回の引当で全属性を得るための非正規化である
/// (裁定 §10-1)。
///
/// 行は値を運ぶだけである。材料から行を組む投影は
/// [`crate::read_tables::ReadTables::project`] が持つ。
///
/// [`StageNode`]: core_command_domain::workflow_definition::StageNode
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionStageRow {
    id: String,
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
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    #[allow(
        clippy::too_many_arguments,
        reason = "表の 1 行の全列を唯一の構築口へ渡す — 列と引数の対応を一覧で読めることを優先する"
    )]
    pub const fn new(
        id: String,
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
    ) -> Self {
        Self {
            id,
            definition_id,
            stage_slug,
            position,
            number,
            name,
            phase,
            execution,
            condition,
            lead_agent,
            support_agents,
            mode,
            for_each,
            workspace_requires,
            produces,
            optional_produces,
            produces_kinds,
            consumes,
            requires_stage,
            sensors,
            scopes,
            reviewer,
            reviewer_max_iterations,
            review_class,
            summary_confirmation,
            plugin,
            enabled,
            gated,
            inputs,
            outputs,
            rules_in_context,
            sensors_applicable,
        }
    }

    /// 主キー — 自然キー (`definition_id`, `stage_slug`) から導いた代理キー。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// `read_definition.id` を指す FK。
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
