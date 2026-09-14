//! `ScopeMetadataView` — `scopes/aidlc-*.md` の frontmatter 1 件の写し (`scope-table` の材料)。
//!
//! scope 定義ファイルは compile コンテキストが読む入力の投影 (リードモデル) であり、クエリ側が
//! 読んで写す (`coding-rules/cqrs-boundaries.md` 規則 6/7)。運ぶのは `scope-table` が読む
//! 3 つ (`name` / `depth` / `testStrategy`) だけで、媒体 (Markdown frontmatter) は DAO 実装の
//! 内部詳細である。
//!
//! 表現は隠し、契約 (アクセサ) だけを公開する (`coding-rules/field-visibility.md`)。

/// scope 定義 1 件のメタ (`scope-table` が読む 3 列)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeMetadataView {
    scope: String,
    depth: String,
    test_strategy: Option<String>,
}

impl ScopeMetadataView {
    /// 3 列をそのまま束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        scope: String,
        depth: String,
        test_strategy: Option<String>,
    ) -> ScopeMetadataView {
        ScopeMetadataView {
            scope,
            depth,
            test_strategy,
        }
    }

    /// scope 名 (frontmatter の `name`)。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// 深さ (frontmatter の `depth`)。
    #[must_use]
    pub fn depth(&self) -> &str {
        &self.depth
    }

    /// テスト戦略 (frontmatter の `testStrategy`、無ければ `None`)。
    #[must_use]
    pub fn test_strategy(&self) -> Option<&str> {
        self.test_strategy.as_deref()
    }
}
