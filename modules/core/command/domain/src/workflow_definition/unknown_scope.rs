//! `UnknownScope` — 定義に載っていないスコープ名の拒否が運ぶ材料。

/// `validScopes()` に無いスコープ名。
///
/// upstream の逐語文言 `Unknown scope: "<scope>". Valid scopes: <csv>` を組み立てるのに
/// 必要な材料をそのまま保持する (文言化は文言カタログ側の責務)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownScope {
    scope: String,
    /// 有効スコープ名 (辞書順)。
    valid_scopes: Vec<String>,
}

impl UnknownScope {
    /// 拒否されたスコープ名と、拒否時点の有効スコープ一覧 (辞書順) を束ねる。
    /// どちらも生値のまま保持する。
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
