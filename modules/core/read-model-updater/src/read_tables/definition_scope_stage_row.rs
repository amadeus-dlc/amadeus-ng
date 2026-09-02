//! `DefinitionScopeStageRow` — `read_definition_scope_stage` の 1 行 (スコープ × ステージの計画)。

use core_command_domain::workflow_definition::{PlanAction, StageSlug, WorkflowDefinitionId};

/// `read_definition_scope_stage` の 1 行。主キーは (`definition_id`, `scope`, `stage_slug`)。
///
/// 値は [`WorkflowDefinition::stages_in_scope`] の答えである。同クエリの `action` は
/// `Option<PlanAction>` — グリッドにそのスコープの列が無い (または列にその slug が無い)
/// ときは答えが無いので、行も NULL にする。**「答えが無い」を EXECUTE や SKIP へ丸めない**。
///
/// `in_scope_order` は EXECUTE の行にだけ付く文書順の連番 (0 始まり) である。
///
/// [`WorkflowDefinition::stages_in_scope`]: core_command_domain::workflow_definition::WorkflowDefinition::stages_in_scope
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionScopeStageRow {
    definition_id: String,
    scope: String,
    stage_slug: String,
    action: Option<String>,
    in_scope_order: Option<usize>,
}

impl DefinitionScopeStageRow {
    /// スコープ × ステージの 1 セルを 1 行へ写す (**この型の唯一の構築経路**)。
    #[must_use]
    pub fn of(
        definition_id: &WorkflowDefinitionId,
        scope: &str,
        stage_slug: &StageSlug,
        action: Option<PlanAction>,
        in_scope_order: Option<usize>,
    ) -> DefinitionScopeStageRow {
        DefinitionScopeStageRow {
            definition_id: definition_id.as_str().to_string(),
            scope: scope.to_string(),
            stage_slug: stage_slug.as_str().to_string(),
            action: action.map(PlanAction::as_str).map(str::to_string),
            in_scope_order,
        }
    }

    /// 定義の系譜 ID。
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
