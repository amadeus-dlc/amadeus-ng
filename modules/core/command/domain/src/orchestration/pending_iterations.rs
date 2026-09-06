//! `PendingIterations` — 判定待ちのレビュー依頼の通し番号の集合 (BR5.5)。

use std::collections::BTreeSet;

use core_infrastructure::collections::FirstClassCollection;

/// 依頼は出したがまだ判定が返っていない通し番号の集合 (昇順・重複なし)。
///
/// [`ReviewAttempt`] の会計のうち「開いている分」を持つ。**クレート内型**であり、ファサード
/// (`orchestration/mod.rs`) からは公開しない — 判定待ちの通し番号は集約の内部会計であって、
/// 外へ出るのは `is_pending(iteration)` という問い合わせの答だけである
/// (`coding-rules/module-visibility.md`)。
///
/// [`ReviewAttempt`]: super::ReviewAttempt
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PendingIterations {
    items: BTreeSet<u32>,
}

impl PendingIterations {
    /// まだ 1 件も依頼していない試行の集合。
    pub(crate) const fn empty() -> PendingIterations {
        PendingIterations {
            items: BTreeSet::new(),
        }
    }

    /// その通し番号を判定待ちに加える (既に待っていれば変化しない)。
    pub(crate) fn with(&mut self, iteration: u32) {
        self.items.insert(iteration);
    }

    /// その通し番号を判定待ちから外す (待っていなければ変化しない)。
    pub(crate) fn without(&mut self, iteration: u32) {
        self.items.remove(&iteration);
    }

    /// その通し番号が判定待ちか。
    pub(crate) fn contains(&self, iteration: u32) -> bool {
        self.items.contains(&iteration)
    }

    /// 判定待ちの件数。
    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }

    /// 判定待ちが 1 件も無いか。
    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 昇順の位置で参照する。範囲外は `None` (panic しない)。走査時間は位置に比例する。
    pub(crate) fn at(&self, index: usize) -> Option<u32> {
        self.items.iter().nth(index).copied()
    }

    /// 昇順に左から畳み込む。空なら初期値を返す。
    pub(crate) fn fold_left<A>(&self, initial: A, mut fold: impl FnMut(A, u32) -> A) -> A {
        self.items
            .iter()
            .fold(initial, |acc, iteration| fold(acc, *iteration))
    }

    /// 条件に一致する通し番号の集合 (昇順のまま)。結果は空になり得る。
    pub(crate) fn filter(&self, mut predicate: impl FnMut(u32) -> bool) -> PendingIterations {
        PendingIterations {
            items: self
                .items
                .iter()
                .filter(|iteration| predicate(**iteration))
                .copied()
                .collect(),
        }
    }
}

impl FirstClassCollection for PendingIterations {
    type Item<'a> = u32;
    type Filtered = Self;
    fn len(&self) -> usize {
        Self::len(self)
    }
    fn is_empty(&self) -> bool {
        Self::is_empty(self)
    }
    fn at(&self, index: usize) -> Option<Self::Item<'_>> {
        Self::at(self, index)
    }
    fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, Self::Item<'a>) -> A) -> A {
        Self::fold_left(self, initial, fold)
    }
    fn filter(&self, predicate: impl FnMut(Self::Item<'_>) -> bool) -> Self {
        Self::filter(self, predicate)
    }
}

#[cfg(test)]
mod tests {
    use super::PendingIterations;
    use core_infrastructure::collections::FirstClassCollection;

    fn iterations(numbers: [u32; 3]) -> PendingIterations {
        let mut pending = PendingIterations::empty();
        for number in numbers {
            pending.with(number);
        }
        pending
    }

    fn numbers(pending: &PendingIterations) -> Vec<u32> {
        pending.fold_left(Vec::new(), |mut acc, iteration| {
            acc.push(iteration);
            acc
        })
    }

    #[test]
    fn a_fresh_attempt_waits_for_nothing() {
        let empty = PendingIterations::empty();
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
        assert_eq!(empty.at(0), None);
        assert!(!empty.contains(1));
        assert_eq!(PendingIterations::default(), empty);
    }

    #[test]
    fn a_request_starts_waiting_and_its_verdict_stops_the_wait() {
        let mut pending = PendingIterations::empty();
        pending.with(1);
        assert!(pending.contains(1));
        assert!(!pending.contains(2));
        pending.without(1);
        assert!(!pending.contains(1));
        assert!(pending.is_empty());
    }

    #[test]
    fn removing_an_absent_iteration_leaves_the_set_alone() {
        let mut pending = PendingIterations::empty();
        pending.with(3);
        pending.without(9);
        assert_eq!(numbers(&pending), [3]);
    }

    #[test]
    fn the_same_iteration_is_only_waited_for_once() {
        let mut pending = PendingIterations::empty();
        pending.with(2);
        pending.with(2);
        assert_eq!(pending.len(), 1);
    }

    #[test]
    fn positions_walk_in_ascending_order_and_stop_at_the_end() {
        let pending = iterations([5, 1, 3]);
        assert_eq!(numbers(&pending), [1, 3, 5]);
        assert_eq!(pending.at(0), Some(1));
        assert_eq!(pending.at(2), Some(5));
        assert_eq!(pending.at(3), None);
        assert_eq!(pending.at(usize::MAX), None);
    }

    #[test]
    fn filtering_keeps_ascending_order_and_can_empty_the_set() {
        let pending = iterations([5, 1, 3]);
        assert_eq!(numbers(&pending.filter(|iteration| iteration > 1)), [3, 5]);
        assert!(pending.filter(|_| false).is_empty());
        assert_eq!(pending.len(), 3, "元の集合は変わらない");
    }

    /// `tests/collection_contract_test.rs` の `check` と同じ検査。この型は
    /// `pub(crate)` でファサード非公開なので、結合テストからは触れずここで固定する。
    #[test]
    fn the_shared_traversal_contract_holds_for_both_cardinalities() {
        for (pending, expected) in [(iterations([5, 1, 3]), 3), (PendingIterations::empty(), 0)] {
            assert_eq!(FirstClassCollection::len(&pending), expected);
            assert_eq!(FirstClassCollection::is_empty(&pending), expected == 0);
            assert_eq!(
                FirstClassCollection::fold_left(&pending, 0, |count, _| count + 1),
                expected
            );
            assert_eq!(
                FirstClassCollection::at(&pending, 0).is_some(),
                expected != 0
            );
            assert!(FirstClassCollection::at(&pending, expected).is_none());
            assert!(FirstClassCollection::at(&pending, usize::MAX).is_none());
            assert_eq!(FirstClassCollection::filter(&pending, |_| true), pending);
            assert!(FirstClassCollection::is_empty(
                &FirstClassCollection::filter(&pending, |_| false)
            ));
        }
    }
}
