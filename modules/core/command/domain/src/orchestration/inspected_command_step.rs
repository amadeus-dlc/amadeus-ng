//! `InspectedCommandStep` — シェルコマンドの実行位置 1 つ分の語。
use super::ScopeToken;
use core_infrastructure::collections::FirstClassCollection;

/// 区切りで分けた実行位置 1 つに並ぶ語 (upstream `ShellWord[]` の 1 区間)。
///
/// 先頭の語がコマンド名、以降が引数である。字句解析そのものはハーネス側の言語拡張
/// (`harness_infrastructure::ReviewerScopeSegments`) が行い、この型は**読んだ結果**
/// だけを持つ。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InspectedCommandStep {
    items: Vec<ScopeToken>,
}
impl InspectedCommandStep {
    /// 語の列を固定する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(items: Vec<ScopeToken>) -> InspectedCommandStep {
        InspectedCommandStep { items }
    }
    /// 語の件数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }
    /// 語が 1 つも無いか。
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
impl FirstClassCollection for InspectedCommandStep {
    type Item<'a> = &'a ScopeToken;
    type Filtered = Self;
    fn len(&self) -> usize {
        InspectedCommandStep::len(self)
    }
    fn at(&self, index: usize) -> Option<&ScopeToken> {
        InspectedCommandStep::at(self, index)
    }
    fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, &'a ScopeToken) -> A) -> A {
        self.items.iter().fold(initial, fold)
    }
    fn filter(&self, mut predicate: impl FnMut(&ScopeToken) -> bool) -> Self {
        InspectedCommandStep::new(
            self.items
                .iter()
                .filter(|word| predicate(word))
                .cloned()
                .collect(),
        )
    }
}
