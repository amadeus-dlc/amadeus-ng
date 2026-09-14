//! `ScopeCatalogRowView` — `scope-table` の 1 行 (scope グリッド + scope 定義メタの結合)。
//!
//! `scope-grid.json` と `scopes/aidlc-*.md` はどちらも compile コンテキストの投影
//! (リードモデル) であり、クエリ側が読んで写す (`coding-rules/cqrs-boundaries.md` 規則 6/7)。
//! 1 行は 2 つの読取源を **scope 名の突合**で組んだもので、組むのはユースケースの仕事である
//! (規則 6 の 2026-09-03 追記 — 「FK をたどって表ごとに引き、View を組む」)。
//!
//! 表現は隠し、契約 (アクセサ) だけを公開する (`coding-rules/field-visibility.md`)。

/// `scope-table` の 1 行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeCatalogRowView {
    scope: String,
    depth: String,
    test_strategy: Option<String>,
    execute: u32,
    total: u32,
}

impl ScopeCatalogRowView {
    /// 5 列をそのまま束ねる (**この型の唯一の構築経路**)。
    ///
    /// `test_strategy` が `None` なのは scope 定義が `testStrategy` を宣言しない場合で、
    /// upstream はそこを `(default)` と描く — その描き分けは出す側の仕事である。
    #[must_use]
    pub const fn new(
        scope: String,
        depth: String,
        test_strategy: Option<String>,
        execute: u32,
        total: u32,
    ) -> ScopeCatalogRowView {
        ScopeCatalogRowView {
            scope,
            depth,
            test_strategy,
            execute,
            total,
        }
    }

    /// scope 名。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// scope 定義が宣言する深さ (`Minimal` / `Standard` / `Comprehensive`)。
    #[must_use]
    pub fn depth(&self) -> &str {
        &self.depth
    }

    /// scope 定義が宣言するテスト戦略 (無ければ `None`)。
    #[must_use]
    pub fn test_strategy(&self) -> Option<&str> {
        self.test_strategy.as_deref()
    }

    /// グリッドで EXECUTE のステージ数。
    #[must_use]
    pub const fn execute(&self) -> u32 {
        self.execute
    }

    /// グリッドに載るステージ総数 (EXECUTE + SKIP)。
    #[must_use]
    pub const fn total(&self) -> u32 {
        self.total
    }
}
