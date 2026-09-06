//! `StageIndexSet` — ステージ位置の集合 (BR5.5)。

use std::collections::BTreeSet;

use core_infrastructure::collections::FirstClassCollection;

use super::stage_index::StageIndex;

/// 文書順の位置 ([`StageIndex`]) の集合 (昇順・重複なし)。
///
/// recompose の反転対象や、jump の読み飛ばし・巻き戻し・承認無効化の対象を 1 つの値として
/// 運ぶ。区間 ([`StageIndexSet::range`]) と和集合・差集合で組み立てるので、集約は `Vec` と
/// range ループを持たない (BR5.5)。
///
/// 空集合を許し和集合が全域なので、`combine` は空集合を単位元とする可換冪等 Monoid になる
/// (`coding-rules/first-class-collections.md` § 結合と差集合)。`divide` は結合の逆演算では
/// なく差集合である。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StageIndexSet {
    items: BTreeSet<StageIndex>,
}

impl StageIndexSet {
    /// 和集合の単位元となる空集合。
    #[must_use]
    pub const fn empty() -> StageIndexSet {
        StageIndexSet {
            items: BTreeSet::new(),
        }
    }

    /// 位置 1 つだけを含む集合。
    #[must_use]
    pub fn singleton(stage: StageIndex) -> StageIndexSet {
        StageIndexSet {
            items: BTreeSet::from([stage]),
        }
    }

    /// 位置の並びを集合にする (重複は畳まれ、昇順に整列する)。
    #[must_use]
    pub fn new(stages: impl IntoIterator<Item = StageIndex>) -> StageIndexSet {
        StageIndexSet {
            items: stages.into_iter().collect(),
        }
    }

    /// 半開区間 `[from, to_exclusive)` の位置集合。前進しない区間は空集合。
    #[must_use]
    pub fn range(from: StageIndex, to_exclusive: StageIndex) -> StageIndexSet {
        StageIndexSet::new((from.to_usize()..to_exclusive.to_usize()).map(StageIndex::new))
    }

    /// その位置を含むか。
    #[must_use]
    pub fn contains(&self, stage: StageIndex) -> bool {
        self.items.contains(&stage)
    }

    /// 位置の個数 (重複がないので集合の濃度そのもの)。
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// 位置を 1 つも含まないか。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 昇順の位置で参照する。範囲外は `None` (panic しない)。走査時間は位置に比例する。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<StageIndex> {
        self.items.iter().nth(index).copied()
    }

    /// 昇順に左から畳み込む。空なら初期値を返す。
    pub fn fold_left<A>(&self, initial: A, mut fold: impl FnMut(A, StageIndex) -> A) -> A {
        self.items
            .iter()
            .fold(initial, |acc, stage| fold(acc, *stage))
    }

    /// 条件に一致する位置の集合 (昇順のまま)。結果は空になり得る。
    #[must_use]
    pub fn filter(&self, mut predicate: impl FnMut(StageIndex) -> bool) -> StageIndexSet {
        StageIndexSet {
            items: self
                .items
                .iter()
                .filter(|stage| predicate(**stage))
                .copied()
                .collect(),
        }
    }

    /// 両方の位置を含む和集合。元の集合は変更しない。
    #[must_use]
    pub fn combine(&self, other: &StageIndexSet) -> StageIndexSet {
        StageIndexSet {
            items: self.items.union(&other.items).copied().collect(),
        }
    }

    /// 他方に含まれる位置を除いた差集合。元の集合は変更しない。
    #[must_use]
    pub fn divide(&self, other: &StageIndexSet) -> StageIndexSet {
        StageIndexSet {
            items: self.items.difference(&other.items).copied().collect(),
        }
    }
}

impl FirstClassCollection for StageIndexSet {
    type Item<'a> = StageIndex;
    type Filtered = Self;
    fn len(&self) -> usize {
        Self::len(self)
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
    use super::StageIndexSet;
    use crate::orchestration::stage_index::StageIndex;
    use core_infrastructure::collections::FirstClassCollection;
    use proptest::prelude::*;
    use std::collections::BTreeSet;

    fn set(positions: [usize; 3]) -> StageIndexSet {
        StageIndexSet::new(positions.into_iter().map(StageIndex::new))
    }

    fn positions(set: &StageIndexSet) -> Vec<usize> {
        set.fold_left(Vec::new(), |mut acc, stage| {
            acc.push(stage.to_usize());
            acc
        })
    }

    #[test]
    fn the_empty_set_carries_no_position() {
        let empty = StageIndexSet::empty();
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
        assert_eq!(empty.at(0), None);
        assert!(!empty.contains(StageIndex::new(0)));
        assert_eq!(StageIndexSet::default(), empty);
    }

    #[test]
    fn a_singleton_carries_exactly_its_position() {
        let only = StageIndexSet::singleton(StageIndex::new(4));
        assert_eq!(only.len(), 1);
        assert!(only.contains(StageIndex::new(4)));
        assert!(!only.contains(StageIndex::new(3)));
        assert_eq!(only.at(0), Some(StageIndex::new(4)));
    }

    #[test]
    fn a_range_is_half_open_and_empty_when_it_does_not_move_forward() {
        assert_eq!(
            positions(&StageIndexSet::range(
                StageIndex::new(1),
                StageIndex::new(4)
            )),
            [1, 2, 3]
        );
        assert!(
            StageIndexSet::range(StageIndex::new(3), StageIndex::new(3)).is_empty(),
            "空区間"
        );
        assert!(
            StageIndexSet::range(StageIndex::new(4), StageIndex::new(1)).is_empty(),
            "逆向きの区間も空"
        );
    }

    #[test]
    fn construction_drops_duplicates_and_orders_by_position() {
        let set = StageIndexSet::new([3usize, 1, 3, 2].into_iter().map(StageIndex::new));
        assert_eq!(positions(&set), [1, 2, 3]);
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn a_position_past_the_end_is_none_and_never_panics() {
        let set = set([5, 2, 9]);
        assert_eq!(set.at(0), Some(StageIndex::new(2)));
        assert_eq!(set.at(2), Some(StageIndex::new(9)));
        assert_eq!(set.at(3), None);
        assert_eq!(set.at(usize::MAX), None);
    }

    #[test]
    fn filtering_keeps_ascending_order_and_can_empty_the_set() {
        let set = set([5, 2, 9]);
        let kept = set.filter(|stage| stage.to_usize() > 2);
        assert_eq!(positions(&kept), [5, 9]);
        assert!(set.filter(|_| false).is_empty());
        assert_eq!(set.len(), 3, "元の集合は変わらない");
    }

    #[test]
    fn union_and_difference_leave_both_inputs_unchanged() {
        let left = set([1, 2, 3]);
        let right = set([3, 4, 5]);
        assert_eq!(positions(&left.combine(&right)), [1, 2, 3, 4, 5]);
        assert_eq!(positions(&left.divide(&right)), [1, 2]);
        assert_eq!(positions(&left), [1, 2, 3]);
        assert_eq!(positions(&right), [3, 4, 5]);
    }

    #[test]
    fn the_shared_traversal_contract_sees_the_same_set() {
        let set = set([5, 2, 9]);
        assert_eq!(FirstClassCollection::len(&set), 3);
        assert!(!FirstClassCollection::is_empty(&set));
        assert_eq!(FirstClassCollection::at(&set, 0), Some(StageIndex::new(2)));
        assert_eq!(FirstClassCollection::at(&set, 3), None);
        assert_eq!(
            FirstClassCollection::fold_left(&set, 0, |count, _| count + 1),
            3
        );
        assert_eq!(FirstClassCollection::filter(&set, |_| true), set);
    }

    fn from_raw(values: BTreeSet<usize>) -> StageIndexSet {
        StageIndexSet::new(values.into_iter().map(StageIndex::new))
    }

    proptest! {
        /// 和集合は空集合を単位元とする可換冪等 Monoid。
        #[test]
        fn union_obeys_monoid_and_set_laws(
            a in proptest::collection::btree_set(0usize..12, 0..6),
            b in proptest::collection::btree_set(0usize..12, 0..6),
            c in proptest::collection::btree_set(0usize..12, 0..6),
        ) {
            let (a, b, c) = (from_raw(a), from_raw(b), from_raw(c));
            prop_assert_eq!(a.combine(&b).combine(&c), a.combine(&b.combine(&c)));
            prop_assert_eq!(&a.combine(&StageIndexSet::empty()), &a);
            prop_assert_eq!(&StageIndexSet::empty().combine(&a), &a);
            prop_assert_eq!(&a.combine(&a), &a);
            prop_assert_eq!(a.combine(&b), b.combine(&a));
        }

        /// 差集合は結合の逆演算ではなく、集合の引き算として振る舞う。
        #[test]
        fn difference_obeys_the_set_laws(
            a in proptest::collection::btree_set(0usize..12, 0..6),
            b in proptest::collection::btree_set(0usize..12, 0..6),
        ) {
            let (a, b) = (from_raw(a), from_raw(b));
            prop_assert_eq!(a.divide(&a), StageIndexSet::empty());
            prop_assert_eq!(&a.divide(&StageIndexSet::empty()), &a);
            let left_over = a.combine(&b).divide(&b);
            prop_assert!(left_over.fold_left(true, |kept, stage| kept && a.contains(stage)));
        }
    }
}
