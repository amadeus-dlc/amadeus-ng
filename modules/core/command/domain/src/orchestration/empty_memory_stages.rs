//! `EmptyMemoryStages` — 日誌が空のまま承認されたと集約が判定した位置の列。

use core_infrastructure::collections::FirstClassCollection;

use crate::workflow_definition::StageSlug;

/// `MEMORY_EMPTY` を記録すると集約が決めた位置の列 (計画順)。
///
/// 判定そのものは集約が持つ — 承認済みか、日誌が空か、同じ承認について既に記録済みかは
/// 集約の状態でしか決まらないからである。投影はこの列を読んで監査行を描くだけであり、
/// 現在状態から判定をやり直さない (`coding-rules/cqrs-boundaries.md`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmptyMemoryStages {
    items: Vec<StageSlug>,
}

impl Default for EmptyMemoryStages {
    fn default() -> Self {
        Self::of_items(Default::default())
    }
}

impl EmptyMemoryStages {
    // 検査済みの列とその部分列は、この構築口で全状態を初期化する。
    const fn of_items(items: Vec<StageSlug>) -> Self {
        Self { items }
    }

    /// 記録対象が 1 つも無い判定。
    #[must_use]
    pub const fn empty() -> EmptyMemoryStages {
        EmptyMemoryStages::of_items(Vec::new())
    }

    /// 計画順のまま列にする (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(items: Vec<StageSlug>) -> EmptyMemoryStages {
        EmptyMemoryStages::of_items(items)
    }

    /// 記録対象の件数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }

    /// 記録対象が 1 つも無いか。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 計画順の添字参照。範囲外は `None` (panic しない)。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&StageSlug> {
        self.items.get(index)
    }

    /// 計画順に左から畳み込む。空なら初期値を返す。
    pub fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, &'a StageSlug) -> A) -> A {
        self.items.iter().fold(initial, fold)
    }

    /// 条件に一致する位置を計画順のまま残す。結果は空になり得る。
    #[must_use]
    pub fn filter(&self, mut predicate: impl FnMut(&StageSlug) -> bool) -> Self {
        EmptyMemoryStages::new(
            self.items
                .iter()
                .filter(|slug| predicate(slug))
                .cloned()
                .collect(),
        )
    }

    /// その位置が記録対象か。
    #[must_use]
    pub fn contains(&self, stage: &StageSlug) -> bool {
        self.items.contains(stage)
    }
}

impl FirstClassCollection for EmptyMemoryStages {
    type Item<'a> = &'a StageSlug;
    type Filtered = Self;
    fn len(&self) -> usize {
        Self::len(self)
    }
    fn at(&self, index: usize) -> Option<&StageSlug> {
        Self::at(self, index)
    }
    fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, &'a StageSlug) -> A) -> A {
        Self::fold_left(self, initial, fold)
    }
    fn filter(&self, predicate: impl FnMut(&StageSlug) -> bool) -> Self {
        Self::filter(self, predicate)
    }
}

#[cfg(test)]
mod tests {
    use super::{EmptyMemoryStages, StageSlug};

    fn slug(name: &str) -> StageSlug {
        StageSlug::parse(name).expect("slug")
    }

    fn stages() -> EmptyMemoryStages {
        EmptyMemoryStages::new(vec![slug("state-init"), slug("reverse-engineering")])
    }

    #[test]
    fn the_empty_decision_names_no_stage() {
        let empty = EmptyMemoryStages::empty();
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
        assert_eq!(empty.at(0), None);
        assert!(!empty.contains(&slug("state-init")));
    }

    #[test]
    fn the_decision_keeps_the_plan_order() {
        let stages = stages();
        assert_eq!(stages.len(), 2);
        assert_eq!(stages.at(0), Some(&slug("state-init")));
        assert_eq!(stages.at(1), Some(&slug("reverse-engineering")));
        assert!(stages.contains(&slug("reverse-engineering")));
        assert_eq!(stages.fold_left(0, |count, _| count + 1), 2);
    }

    #[test]
    fn filtering_keeps_the_order_and_may_become_empty() {
        let only = stages().filter(|slug| slug.as_str() == "state-init");
        assert_eq!(only.len(), 1);
        assert_eq!(only.at(0), Some(&slug("state-init")));
        assert!(stages().filter(|_| false).is_empty());
    }
}
