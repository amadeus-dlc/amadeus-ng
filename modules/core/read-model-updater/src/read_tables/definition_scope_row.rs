//! `DefinitionScopeRow` — `read_definition_scope` の 1 行 (スコープ 1 件のメタデータと費用)。

use core_command_domain::workflow_definition::{
    ReviewCapValue, ScopeCost, ScopeMetadata, SkeletonDefault, WorkflowDefinition,
};

use super::json_column;
use super::row_id;

/// `read_definition_scope` の 1 行。主キーは 1 列 `id` (自然キー
/// (`definition_id`, `scope`) から導いた代理キー)。`definition_id` は `read_definition.id`
/// を指す FK である。
///
/// 費用 4 列は [`WorkflowDefinition::scope_cost`] の答えである。グリッド列を持たない
/// 有効スコープでは答えが `None` になるので、4 列とも NULL になる (`has_grid_column` が
/// その理由を語る)。
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
}

impl DefinitionScopeRow {
    /// スコープ 1 件を 1 行へ写す (**この型の唯一の構築経路**)。
    #[must_use]
    pub fn of(
        definition: &WorkflowDefinition,
        scope: &str,
        metadata: &ScopeMetadata,
    ) -> DefinitionScopeRow {
        let cost = definition.scope_cost(scope);
        DefinitionScopeRow {
            id: row_id::definition_scope(definition.id().as_str(), scope),
            definition_id: definition.id().as_str().to_string(),
            scope: scope.to_string(),
            depth: metadata.depth().map(str::to_string),
            keywords: json_column::strings(metadata.keywords()),
            skeleton: metadata
                .skeleton()
                .map(SkeletonDefault::as_str)
                .map(str::to_string),
            review_cap: metadata
                .review_cap()
                .map(ReviewCapValue::as_str)
                .map(str::to_string),
            freeform_default: metadata.freeform_default(),
            has_grid_column: definition.grid().contains_scope(scope),
            cost_total: cost.as_ref().map(ScopeCost::total),
            cost_execute: cost.as_ref().map(ScopeCost::execute),
            cost_gates: cost.as_ref().map(ScopeCost::gates),
            cost_per_unit_stages: cost.as_ref().map(ScopeCost::per_unit_stages),
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
}
