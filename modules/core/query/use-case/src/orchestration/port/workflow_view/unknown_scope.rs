//! `UnknownScope` — [`DefinitionView::subgraph_for_scope`] が拒否した未知スコープ。
//!
//! upstream の逐語文言を組み立てるのに必要な材料 (拒否されたスコープ名と、拒否時点の有効
//! スコープ一覧) をそのまま保持する。文言化は出す側の責務である
//! (`coding-rules/error-handling.md`)。
//!
//! [`DefinitionView::subgraph_for_scope`]: super::DefinitionView::subgraph_for_scope

use std::fmt;

/// `validScopes()` に無いスコープ名。
///
/// upstream の逐語文言 `Unknown scope: "<scope>". Valid scopes: <csv>` を組み立てるのに
/// 必要な材料をそのまま保持する (文言化は出す側の責務)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownScope {
    scope: String,
    valid_scopes: Vec<String>,
}

impl UnknownScope {
    /// 拒否されたスコープ名と、拒否時点の有効スコープ一覧 (辞書順) を束ねる。
    #[must_use]
    pub fn new(scope: impl Into<String>, valid_scopes: Vec<String>) -> UnknownScope {
        UnknownScope {
            scope: scope.into(),
            valid_scopes,
        }
    }

    /// 拒否されたスコープ名。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// 有効スコープ名 (辞書順)。
    #[must_use]
    pub fn valid_scopes(&self) -> &[String] {
        &self.valid_scopes
    }
}

impl fmt::Display for UnknownScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown scope {:?}; valid scopes: {}",
            self.scope,
            self.valid_scopes.join(", ")
        )
    }
}

impl std::error::Error for UnknownScope {}
