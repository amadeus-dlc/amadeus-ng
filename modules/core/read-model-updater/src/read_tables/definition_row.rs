//! `DefinitionRow` — `read_definition` の 1 行 (定義 1 件の要約)。

use core_command_domain::workflow_definition::WorkflowDefinition;

/// `read_definition` の 1 行。主キーは `definition_id`。
///
/// 値はすべて再生した [`WorkflowDefinition`] のクエリの答えの写しである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionRow {
    definition_id: String,
    revision: String,
    stage_count: usize,
    scope_count: usize,
}

impl DefinitionRow {
    /// 再生した定義を 1 行へ写す (**この型の唯一の構築経路**)。
    #[must_use]
    pub fn of(definition: &WorkflowDefinition) -> DefinitionRow {
        DefinitionRow {
            definition_id: definition.id().as_str().to_string(),
            revision: definition.revision().as_str().to_string(),
            stage_count: definition.graph().len(),
            scope_count: definition.scopes().len(),
        }
    }

    /// 定義の系譜 ID。
    #[must_use]
    pub fn definition_id(&self) -> &str {
        &self.definition_id
    }

    /// 内容版 (`sha256:` 接頭の 64 桁)。
    #[must_use]
    pub fn revision(&self) -> &str {
        &self.revision
    }

    /// グラフのノード数。
    #[must_use]
    pub const fn stage_count(&self) -> usize {
        self.stage_count
    }

    /// スコープカタログの件数 (有効スコープの権威)。
    #[must_use]
    pub const fn scope_count(&self) -> usize {
        self.scope_count
    }
}
