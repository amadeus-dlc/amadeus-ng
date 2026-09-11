//! `InspectedCommand` — 検査するシェルコマンド 1 本 (実行位置の列)。
use super::InspectedCommandStep;
use core_infrastructure::collections::FirstClassCollection;

/// `Bash` の 1 呼出しを実行位置へ分けたもの (upstream `shellSegments` の結果)。
///
/// 実行はしない。`cd` が続く実行位置の基点を動かすので**順序に意味がある**。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InspectedCommand {
    items: Vec<InspectedCommandStep>,
}
impl InspectedCommand {
    /// 実行位置の列を固定する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(items: Vec<InspectedCommandStep>) -> InspectedCommand {
        InspectedCommand { items }
    }
    /// 実行位置の件数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }
    /// 実行位置が 1 つも無いか。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    /// 出現順の添字参照。範囲外は `None`。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&InspectedCommandStep> {
        self.items.get(index)
    }
}
impl FirstClassCollection for InspectedCommand {
    type Item<'a> = &'a InspectedCommandStep;
    type Filtered = Self;
    fn len(&self) -> usize {
        InspectedCommand::len(self)
    }
    fn at(&self, index: usize) -> Option<&InspectedCommandStep> {
        InspectedCommand::at(self, index)
    }
    fn fold_left<'a, A>(
        &'a self,
        initial: A,
        fold: impl FnMut(A, &'a InspectedCommandStep) -> A,
    ) -> A {
        self.items.iter().fold(initial, fold)
    }
    fn filter(&self, mut predicate: impl FnMut(&InspectedCommandStep) -> bool) -> Self {
        InspectedCommand::new(
            self.items
                .iter()
                .filter(|step| predicate(step))
                .cloned()
                .collect(),
        )
    }
}
