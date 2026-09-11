//! `ExemptPaths` — 差し向け記録が現在の Unit の外に許した経路の列。
use super::ScopeToken;
use core_infrastructure::collections::FirstClassCollection;

/// レビュアーが現在の Unit の外で触ってよい経路 (upstream `ReviewerDispatch.exempt`)。
///
/// 中身は差し向け記録が並べた綴りそのもので、`construction/` を通らない項目は
/// 照合に効かない (`construction/` の外は常に許可されるため)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExemptPaths {
    items: Vec<ScopeToken>,
}
impl ExemptPaths {
    /// 許可経路の列を固定する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(items: Vec<ScopeToken>) -> ExemptPaths {
        ExemptPaths { items }
    }
    /// 許可経路の件数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }
    /// 許可経路が 1 つも無いか。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    /// 出現順の添字参照。範囲外は `None`。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&ScopeToken> {
        self.items.get(index)
    }
}
impl FirstClassCollection for ExemptPaths {
    type Item<'a> = &'a ScopeToken;
    type Filtered = Self;
    fn len(&self) -> usize {
        ExemptPaths::len(self)
    }
    fn at(&self, index: usize) -> Option<&ScopeToken> {
        ExemptPaths::at(self, index)
    }
    fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, &'a ScopeToken) -> A) -> A {
        self.items.iter().fold(initial, fold)
    }
    fn filter(&self, mut predicate: impl FnMut(&ScopeToken) -> bool) -> Self {
        ExemptPaths::new(
            self.items
                .iter()
                .filter(|path| predicate(path))
                .cloned()
                .collect(),
        )
    }
}
