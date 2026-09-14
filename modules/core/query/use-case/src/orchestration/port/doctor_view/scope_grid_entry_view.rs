//! `scope-grid.json` の 1 スコープの写し。

/// スコープ名と `slug → EXECUTE | SKIP` の並び。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeGridEntryView {
    scope: String,
    stages: Vec<(String, String)>,
}

impl ScopeGridEntryView {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(scope: String, stages: Vec<(String, String)>) -> Self {
        Self { scope, stages }
    }

    /// スコープ名。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// `(slug, action)` の並び (ファイル順)。
    #[must_use]
    pub fn stages(&self) -> &[(String, String)] {
        &self.stages
    }
}
