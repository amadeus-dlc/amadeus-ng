//! `StageSlugSet` — ステージ slug の集合 (BR5.5)。

use std::collections::BTreeSet;

use core_infrastructure::collections::FirstClassCollection;

use crate::workflow_definition::StageSlug;

/// ステージ slug の集合 (辞書順・重複なし)。
///
/// `Recomposed` の `skipped` / `added` が運ぶ反転対象である。位置集合 ([`StageIndexSet`]) を
/// 添字帳で写して作る ([`StageEntries::slugs_at`]) ので、recompose 入力の重複はここで畳まれる。
///
/// 空集合を許し和集合が全域なので、`combine` は空集合を単位元とする可換冪等 Monoid になる
/// (`coding-rules/first-class-collections.md` § 結合と差集合)。`divide` は結合の逆演算では
/// なく差集合である。
///
/// **並び順は辞書順であり文書順ではない** — 監査行と状態ファイルの逐語一致 (NFR1) が要る
/// 投影側は、この集合を計画の位置で並べ直してから描く。
///
/// [`StageIndexSet`]: super::StageIndexSet
/// [`StageEntries::slugs_at`]: super::StageEntries::slugs_at
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StageSlugSet {
    items: BTreeSet<StageSlug>,
}

impl StageSlugSet {
    /// 和集合の単位元となる空集合。
    #[must_use]
    pub const fn empty() -> StageSlugSet {
        StageSlugSet {
            items: BTreeSet::new(),
        }
    }

    /// slug の並びを集合にする (重複は畳まれ、辞書順に整列する)。
    #[must_use]
    pub fn new(slugs: impl IntoIterator<Item = StageSlug>) -> StageSlugSet {
        StageSlugSet {
            items: slugs.into_iter().collect(),
        }
    }

    /// その slug を含むか。
    #[must_use]
    pub fn contains(&self, slug: &StageSlug) -> bool {
        self.items.contains(slug)
    }

    /// slug の個数 (重複がないので集合の濃度そのもの)。
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// slug を 1 つも含まないか。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 辞書順の位置で参照する。範囲外は `None` (panic しない)。走査時間は位置に比例する。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&StageSlug> {
        self.items.iter().nth(index)
    }

    /// 辞書順に左から畳み込む。空なら初期値を返す。
    pub fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, &'a StageSlug) -> A) -> A {
        self.items.iter().fold(initial, fold)
    }

    /// 条件に一致する slug の集合 (辞書順のまま)。結果は空になり得る。
    #[must_use]
    pub fn filter(&self, mut predicate: impl FnMut(&StageSlug) -> bool) -> StageSlugSet {
        StageSlugSet {
            items: self
                .items
                .iter()
                .filter(|slug| predicate(slug))
                .cloned()
                .collect(),
        }
    }

    /// 両方の slug を含む和集合。元の集合は変更しない。
    #[must_use]
    pub fn combine(&self, other: &StageSlugSet) -> StageSlugSet {
        StageSlugSet {
            items: self.items.union(&other.items).cloned().collect(),
        }
    }

    /// 他方に含まれる slug を除いた差集合。元の集合は変更しない。
    #[must_use]
    pub fn divide(&self, other: &StageSlugSet) -> StageSlugSet {
        StageSlugSet {
            items: self.items.difference(&other.items).cloned().collect(),
        }
    }
}

impl FirstClassCollection for StageSlugSet {
    type Item<'a> = &'a StageSlug;
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
    use super::StageSlugSet;
    use crate::workflow_definition::StageSlug;
    use core_infrastructure::collections::FirstClassCollection;
    use proptest::prelude::*;
    use std::collections::BTreeSet;

    fn slug(name: &str) -> StageSlug {
        StageSlug::parse(name).unwrap()
    }

    fn set(names: [&str; 3]) -> StageSlugSet {
        StageSlugSet::new(names.into_iter().map(slug))
    }

    fn names(set: &StageSlugSet) -> Vec<String> {
        set.fold_left(Vec::new(), |mut acc, slug| {
            acc.push(slug.as_str().to_string());
            acc
        })
    }

    #[test]
    fn the_empty_set_carries_no_slug() {
        let empty = StageSlugSet::empty();
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
        assert_eq!(empty.at(0), None);
        assert!(!empty.contains(&slug("intent-capture")));
        assert_eq!(StageSlugSet::default(), empty);
    }

    #[test]
    fn construction_drops_duplicates_and_orders_lexicographically() {
        let set = StageSlugSet::new(
            [
                "user-stories",
                "feasibility",
                "user-stories",
                "market-research",
            ]
            .into_iter()
            .map(slug),
        );
        assert_eq!(
            names(&set),
            ["feasibility", "market-research", "user-stories"]
        );
        assert_eq!(set.len(), 3);
        assert!(set.contains(&slug("feasibility")));
        assert!(!set.contains(&slug("scope-definition")));
    }

    #[test]
    fn a_position_past_the_end_is_none_and_never_panics() {
        let set = set(["c-stage", "a-stage", "b-stage"]);
        assert_eq!(set.at(0), Some(&slug("a-stage")));
        assert_eq!(set.at(2), Some(&slug("c-stage")));
        assert_eq!(set.at(3), None);
        assert_eq!(set.at(usize::MAX), None);
    }

    #[test]
    fn filtering_keeps_lexicographic_order_and_can_empty_the_set() {
        let set = set(["c-stage", "a-stage", "b-stage"]);
        let kept = set.filter(|slug| slug.as_str() != "b-stage");
        assert_eq!(names(&kept), ["a-stage", "c-stage"]);
        assert!(set.filter(|_| false).is_empty());
        assert_eq!(set.len(), 3, "元の集合は変わらない");
    }

    #[test]
    fn union_and_difference_leave_both_inputs_unchanged() {
        let left = set(["a-stage", "b-stage", "c-stage"]);
        let right = set(["c-stage", "d-stage", "e-stage"]);
        assert_eq!(
            names(&left.combine(&right)),
            ["a-stage", "b-stage", "c-stage", "d-stage", "e-stage"]
        );
        assert_eq!(names(&left.divide(&right)), ["a-stage", "b-stage"]);
        assert_eq!(names(&left).len(), 3);
        assert_eq!(names(&right).len(), 3);
    }

    #[test]
    fn the_shared_traversal_contract_sees_the_same_set() {
        let set = set(["c-stage", "a-stage", "b-stage"]);
        assert_eq!(FirstClassCollection::len(&set), 3);
        assert!(!FirstClassCollection::is_empty(&set));
        assert_eq!(FirstClassCollection::at(&set, 0), Some(&slug("a-stage")));
        assert_eq!(FirstClassCollection::at(&set, 3), None);
        assert_eq!(
            FirstClassCollection::fold_left(&set, 0, |count, _| count + 1),
            3
        );
        assert_eq!(FirstClassCollection::filter(&set, |_| true), set);
    }

    fn from_raw(values: BTreeSet<String>) -> StageSlugSet {
        StageSlugSet::new(values.into_iter().map(|name| slug(&name)))
    }

    proptest! {
        /// 和集合は空集合を単位元とする可換冪等 Monoid。
        #[test]
        fn union_obeys_monoid_and_set_laws(
            a in proptest::collection::btree_set("[a-z]{1,4}", 0..6),
            b in proptest::collection::btree_set("[a-z]{1,4}", 0..6),
            c in proptest::collection::btree_set("[a-z]{1,4}", 0..6),
        ) {
            let (a, b, c) = (from_raw(a), from_raw(b), from_raw(c));
            prop_assert_eq!(a.combine(&b).combine(&c), a.combine(&b.combine(&c)));
            prop_assert_eq!(&a.combine(&StageSlugSet::empty()), &a);
            prop_assert_eq!(&StageSlugSet::empty().combine(&a), &a);
            prop_assert_eq!(&a.combine(&a), &a);
            prop_assert_eq!(a.combine(&b), b.combine(&a));
        }

        /// 差集合は結合の逆演算ではなく、集合の引き算として振る舞う。
        #[test]
        fn difference_obeys_the_set_laws(
            a in proptest::collection::btree_set("[a-z]{1,4}", 0..6),
            b in proptest::collection::btree_set("[a-z]{1,4}", 0..6),
        ) {
            let (a, b) = (from_raw(a), from_raw(b));
            prop_assert_eq!(a.divide(&a), StageSlugSet::empty());
            prop_assert_eq!(&a.divide(&StageSlugSet::empty()), &a);
            let left_over = a.combine(&b).divide(&b);
            prop_assert!(left_over.fold_left(true, |kept, slug| kept && a.contains(slug)));
        }
    }
}
