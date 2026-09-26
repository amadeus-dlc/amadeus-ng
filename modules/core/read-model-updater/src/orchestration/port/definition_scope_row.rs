//! `DefinitionScopeRow` — `read_definition_scope` の 1 行 (スコープ 1 件のメタデータと費用)。

/// `read_definition_scope` の 1 行。主キーは 1 列 `id` (自然キー
/// (`definition_id`, `scope`) から導いた代理キー)。`definition_id` は `read_definition.id`
/// を指す FK である。
///
/// 費用 4 列は [`WorkflowDefinition::scope_cost`] の答えである。グリッド列を持たない
/// 有効スコープでは答えが `None` になるので、4 列とも NULL になる (`has_grid_column` が
/// その理由を語る)。`greenfield_cost_*` の 4 列は同じ問いを greenfield のワークスペース向け
/// (`reverse-engineering` を畳んだ実効値 — upstream `effectiveScopeCostSummary`) に答えた
/// もので、読み手は観測したプロジェクト種別でどちらの列を読むかを選ぶだけである。
///
/// 行は値を運ぶだけである。材料から行を組む投影は
/// [`crate::read_tables::ReadTables::project`] が持つ。
///
/// [`WorkflowDefinition::scope_cost`]: core_command_domain::workflow_definition::WorkflowDefinition::scope_cost
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionScopeRow {
    id: String,
    definition_id: String,
    scope: String,
    depth: Option<String>,
    keywords: String,
    skeleton: Option<String>,
    review_cap: Option<String>,
    freeform_default: bool,
    has_grid_column: bool,
    cost_total: Option<usize>,
    cost_execute: Option<usize>,
    cost_gates: Option<usize>,
    cost_per_unit_stages: Option<usize>,
    greenfield_cost_total: Option<usize>,
    greenfield_cost_execute: Option<usize>,
    greenfield_cost_gates: Option<usize>,
    greenfield_cost_per_unit_stages: Option<usize>,
}

impl DefinitionScopeRow {
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    #[allow(
        clippy::too_many_arguments,
        reason = "表の 1 行の全列を唯一の構築口へ渡す — 列と引数の対応を一覧で読めることを優先する"
    )]
    pub const fn new(
        id: String,
        definition_id: String,
        scope: String,
        depth: Option<String>,
        keywords: String,
        skeleton: Option<String>,
        review_cap: Option<String>,
        freeform_default: bool,
        has_grid_column: bool,
        cost_total: Option<usize>,
        cost_execute: Option<usize>,
        cost_gates: Option<usize>,
        cost_per_unit_stages: Option<usize>,
        greenfield_cost_total: Option<usize>,
        greenfield_cost_execute: Option<usize>,
        greenfield_cost_gates: Option<usize>,
        greenfield_cost_per_unit_stages: Option<usize>,
    ) -> Self {
        Self {
            id,
            definition_id,
            scope,
            depth,
            keywords,
            skeleton,
            review_cap,
            freeform_default,
            has_grid_column,
            cost_total,
            cost_execute,
            cost_gates,
            cost_per_unit_stages,
            greenfield_cost_total,
            greenfield_cost_execute,
            greenfield_cost_gates,
            greenfield_cost_per_unit_stages,
        }
    }

    /// 主キー — 自然キー (`definition_id`, `scope`) から導いた代理キー。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// `read_definition.id` を指す FK。
    #[must_use]
    pub fn definition_id(&self) -> &str {
        &self.definition_id
    }

    /// スコープ名。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// 成果物の詳細度 (宣言が無ければ NULL)。
    #[must_use]
    pub fn depth(&self) -> Option<&str> {
        self.depth.as_deref()
    }

    /// スコープ選定のキーワードの 1 行 JSON 配列。
    #[must_use]
    pub fn keywords(&self) -> &str {
        &self.keywords
    }

    /// walking skeleton の既定 (宣言が無ければ NULL)。
    #[must_use]
    pub fn skeleton(&self) -> Option<&str> {
        self.skeleton.as_deref()
    }

    /// レビュー階級の上限 (宣言が無ければ NULL)。
    #[must_use]
    pub fn review_cap(&self) -> Option<&str> {
        self.review_cap.as_deref()
    }

    /// 自由記述を既定とするか。
    #[must_use]
    pub const fn freeform_default(&self) -> bool {
        self.freeform_default
    }

    /// グリッドにこのスコープの列が在るか (無くても有効スコープではある)。
    #[must_use]
    pub const fn has_grid_column(&self) -> bool {
        self.has_grid_column
    }

    /// 列に載っているステージ総数。
    #[must_use]
    pub const fn cost_total(&self) -> Option<usize> {
        self.cost_total
    }

    /// EXECUTE のステージ数。
    #[must_use]
    pub const fn cost_execute(&self) -> Option<usize> {
        self.cost_execute
    }

    /// 承認ゲートの数。
    #[must_use]
    pub const fn cost_gates(&self) -> Option<usize> {
        self.cost_gates
    }

    /// unit 反復するステージの数。
    #[must_use]
    pub const fn cost_per_unit_stages(&self) -> Option<usize> {
        self.cost_per_unit_stages
    }

    /// greenfield 向けの実効費用 — 列に載っているステージ総数 (名目値と同じ)。
    #[must_use]
    pub const fn greenfield_cost_total(&self) -> Option<usize> {
        self.greenfield_cost_total
    }

    /// greenfield 向けの実効費用 — EXECUTE のステージ数。
    #[must_use]
    pub const fn greenfield_cost_execute(&self) -> Option<usize> {
        self.greenfield_cost_execute
    }

    /// greenfield 向けの実効費用 — 承認ゲートの数。
    #[must_use]
    pub const fn greenfield_cost_gates(&self) -> Option<usize> {
        self.greenfield_cost_gates
    }

    /// greenfield 向けの実効費用 — unit 反復するステージの数。
    #[must_use]
    pub const fn greenfield_cost_per_unit_stages(&self) -> Option<usize> {
        self.greenfield_cost_per_unit_stages
    }
}
