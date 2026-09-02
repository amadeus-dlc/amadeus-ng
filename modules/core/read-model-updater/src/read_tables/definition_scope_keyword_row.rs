//! `DefinitionScopeKeywordRow` — `read_definition_scope_keyword` の 1 行 (語からスコープへの逆引き)。

use core_command_domain::workflow_definition::WorkflowDefinitionId;

/// `read_definition_scope_keyword` の 1 行。主キーは (`definition_id`, `keyword`)。
///
/// スコープ側のカタログ (`scopes`) は「スコープ → 語の並び」だが、スコープ検出が要るのは
/// 逆向きの「語 → スコープ」である。同じ語を複数のスコープが宣言したときは**スコープ名の
/// 辞書順で最初の 1 つ**が行になる (辞書順の先着は選択ではなく決定的な畳み込みである)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionScopeKeywordRow {
    definition_id: String,
    keyword: String,
    scope: String,
}

impl DefinitionScopeKeywordRow {
    /// 語とその先着スコープを 1 行へ写す (**この型の唯一の構築経路**)。
    #[must_use]
    pub fn of(
        definition_id: &WorkflowDefinitionId,
        keyword: &str,
        scope: &str,
    ) -> DefinitionScopeKeywordRow {
        DefinitionScopeKeywordRow {
            definition_id: definition_id.as_str().to_string(),
            keyword: keyword.to_string(),
            scope: scope.to_string(),
        }
    }

    /// 定義の系譜 ID。
    #[must_use]
    pub fn definition_id(&self) -> &str {
        &self.definition_id
    }

    /// スコープ選定の語。
    #[must_use]
    pub fn keyword(&self) -> &str {
        &self.keyword
    }

    /// その語を宣言したスコープ (辞書順で最初のもの)。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }
}
