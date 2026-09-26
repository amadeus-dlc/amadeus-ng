//! `DefinitionScopeStageRow` — `read_definition_scope_stage` の 1 行 (スコープ × ステージの計画)。

/// `read_definition_scope_stage` の 1 行。主キーは 1 列 `id` (自然キー
/// (`definition_id`, `scope`, `stage_slug`) から導いた代理キー)。`definition_id` は
/// `read_definition.id` を指す FK である。
///
/// 値は [`WorkflowDefinition::stages_in_scope`] の答えである。同クエリの `action` は
/// `Option<PlanAction>` — グリッドにそのスコープの列が無い (または列にその slug が無い)
/// ときは答えが無いので、行も NULL にする。**「答えが無い」を EXECUTE や SKIP へ丸めない**。
///
/// `in_scope_order` は EXECUTE の行にだけ付く文書順の連番 (0 始まり) である。
///
/// 行は値を運ぶだけである。材料から行を組む投影は
/// [`crate::read_tables::ReadTables::project`] が持つ。
///
/// [`WorkflowDefinition::stages_in_scope`]: core_command_domain::workflow_definition::WorkflowDefinition::stages_in_scope
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionScopeStageRow {
    id: String,
    definition_id: String,
    scope: String,
    stage_slug: String,
    action: Option<String>,
    in_scope_order: Option<usize>,
}

impl DefinitionScopeStageRow {
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        id: String,
        definition_id: String,
        scope: String,
        stage_slug: String,
        action: Option<String>,
        in_scope_order: Option<usize>,
    ) -> Self {
        Self {
            id,
            definition_id,
            scope,
            stage_slug,
            action,
            in_scope_order,
        }
    }

    /// 主キー — 自然キー (`definition_id`, `scope`, `stage_slug`) から導いた代理キー。
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

    /// ステージの slug。
    #[must_use]
    pub fn stage_slug(&self) -> &str {
        &self.stage_slug
    }

    /// 静的グリッドの計画 (`EXECUTE` / `SKIP`。列が無ければ NULL)。
    #[must_use]
    pub fn action(&self) -> Option<&str> {
        self.action.as_deref()
    }

    /// EXECUTE のステージだけに付く文書順の連番 (0 始まり)。
    #[must_use]
    pub const fn in_scope_order(&self) -> Option<usize> {
        self.in_scope_order
    }
}
