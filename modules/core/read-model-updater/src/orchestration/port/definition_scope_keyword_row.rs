//! `DefinitionScopeKeywordRow` — `read_definition_scope_keyword` の 1 行 (語からスコープへの逆引き)。

/// `read_definition_scope_keyword` の 1 行。主キーは 1 列 `id` (自然キー
/// (`definition_id`, `keyword`) から導いた代理キー)。`definition_id` は
/// `read_definition.id` を指す FK である。
///
/// スコープ側のカタログ (`scopes`) は「スコープ → 語の並び」だが、スコープ検出が要るのは
/// 逆向きの「語 → スコープ」である。同じ語を複数のスコープが宣言したときは**スコープ名の
/// 辞書順で最初の 1 つ**が行になる (辞書順の先着は選択ではなく決定的な畳み込みである)。
///
/// 行は値を運ぶだけである。材料から行を組む投影は
/// [`crate::read_tables::ReadTables::project`] が持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionScopeKeywordRow {
    id: String,
    definition_id: String,
    keyword: String,
    scope: String,
}

impl DefinitionScopeKeywordRow {
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(id: String, definition_id: String, keyword: String, scope: String) -> Self {
        Self {
            id,
            definition_id,
            keyword,
            scope,
        }
    }

    /// 主キー — 自然キー (`definition_id`, `keyword`) から導いた代理キー。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// `read_definition.id` を指す FK。
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
