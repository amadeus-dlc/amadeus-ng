//! `BoltRefs` — 単一行リスト値。空は常に `[empty list]`、非空はソート済みブラケットリスト
//! (round-trip 決定的)。append/remove は重複・不在で **Err** (無言 no-op しない)
//! (upstream `aidlc-lib.ts:6635-6662`, 03 §5.2)。

use std::collections::BTreeSet;
use std::fmt;

use super::bolt_refs_error::BoltRefsError;

/// 空リストの唯一の放出形。`parse` は `""` もこのリテラルも空として受理するが、`emit` は常に
/// こちらを書く (round-trip 決定性の要)。
pub const EMPTY_LIST_LITERAL: &str = "[empty list]";

/// `Bolt Refs` フィールドの値 — 進行中 Bolt の slug 集合 (fork で追加、merge で除去)。
/// 重複を持たず、放出順は入力順によらず整列。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BoltRefs(BTreeSet<String>);

impl BoltRefs {
    /// 和集合の単位元となる空集合。
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// 両方の参照を含む和集合。重複を除き辞書順に並べる。
    #[must_use]
    pub fn combine(&self, other: &Self) -> Self {
        Self(self.0.union(&other.0).cloned().collect())
    }

    /// 他方に含まれる参照を除いた差集合。元の集合は変更しない。
    #[must_use]
    pub fn divide(&self, other: &Self) -> Self {
        Self(self.0.difference(&other.0).cloned().collect())
    }

    /// 条件に一致する参照の集合。
    #[must_use]
    pub fn filter(&self, mut predicate: impl FnMut(&str) -> bool) -> Self {
        Self(
            self.0
                .iter()
                .filter(|slug| predicate(slug))
                .cloned()
                .collect(),
        )
    }

    /// 参照を変換し、重複を除いて辞書順の集合を作る。
    ///
    /// # Errors
    ///
    /// 変換結果が空・空集合の予約語・前後空白・改行・リストの区切り文字を含む場合はMalformed。
    /// 元の集合は変換の成功・失敗にかかわらず変更しない。
    pub fn map(&self, mut transform: impl FnMut(&str) -> String) -> Result<Self, BoltRefsError> {
        let mut mapped = BTreeSet::new();
        for slug in &self.0 {
            let next = transform(slug);
            if next.is_empty()
                || next == "empty list"
                || next.trim() != next
                || next.contains([',', '[', ']', '\n', '\r'])
            {
                return Err(BoltRefsError::Malformed(next));
            }
            mapped.insert(next);
        }
        Ok(Self(mapped))
    }

    /// 辞書順に左から畳み込む。空なら初期値を返す。
    pub fn fold_left<'a, A>(&'a self, initial: A, mut fold: impl FnMut(A, &'a str) -> A) -> A {
        self.0.iter().fold(initial, |acc, slug| fold(acc, slug))
    }

    /// 辞書順の位置で参照する。範囲外はNone。走査時間は位置に比例する。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&str> {
        self.0.iter().nth(index).map(String::as_str)
    }

    /// 受理形: `""` / `[empty list]` / `[a, b]` (upstream `parseRefsList`)。
    /// # Errors
    ///
    /// ブラケット不整合・空要素は `Malformed`。
    pub fn parse(s: &str) -> Result<BoltRefs, BoltRefsError> {
        let t = s.trim();
        if t.is_empty() || t == EMPTY_LIST_LITERAL {
            return Ok(BoltRefs::default());
        }
        let inner = t
            .strip_prefix('[')
            .and_then(|x| x.strip_suffix(']'))
            .ok_or_else(|| BoltRefsError::Malformed(s.to_string()))?;
        let mut set = BTreeSet::new();
        for part in inner.split(',') {
            let slug = part.trim();
            if slug.is_empty() {
                return Err(BoltRefsError::Malformed(s.to_string()));
            }
            set.insert(slug.to_string());
        }
        Ok(BoltRefs(set))
    }

    /// 放出形は決定的: 空 → `[empty list]`、非空 → ソート済み `[a, b]` (upstream `emitRefsList`)。
    pub fn emit(&self) -> String {
        if self.0.is_empty() {
            EMPTY_LIST_LITERAL.to_string()
        } else {
            let joined: Vec<&str> = self.0.iter().map(String::as_str).collect();
            format!("[{}]", joined.join(", "))
        }
    }

    /// # Errors
    ///
    /// 重複 slug は `DuplicateSlug` (無言 no-op しない)。
    pub fn append_slug(&mut self, slug: &str) -> Result<(), BoltRefsError> {
        if !self.0.insert(slug.to_string()) {
            return Err(BoltRefsError::DuplicateSlug(slug.to_string()));
        }
        Ok(())
    }

    /// # Errors
    ///
    /// 不在 slug は `MissingSlug` (無言 no-op しない)。
    pub fn remove_slug(&mut self, slug: &str) -> Result<(), BoltRefsError> {
        if !self.0.remove(slug) {
            return Err(BoltRefsError::MissingSlug(slug.to_string()));
        }
        Ok(())
    }

    /// slug の在否 — `append_slug` / `remove_slug` が拒否するかの事前判定。
    #[must_use]
    pub fn contains(&self, slug: &str) -> bool {
        self.0.contains(slug)
    }

    /// 保持している slug の個数 (重複がないので集合の濃度そのもの)。
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// 空か — 空は `emit` で `[empty list]` になる (空文字列は書かない)。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for BoltRefs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.emit())
    }
}

impl core_infrastructure::collections::FirstClassCollection for BoltRefs {
    type Item<'a> = &'a str;
    type Filtered = Self;
    fn len(&self) -> usize {
        Self::len(self)
    }
    fn at(&self, index: usize) -> Option<&str> {
        Self::at(self, index)
    }
    fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, &'a str) -> A) -> A {
        Self::fold_left(self, initial, fold)
    }
    fn filter(&self, predicate: impl FnMut(&str) -> bool) -> Self {
        Self::filter(self, predicate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn collection_operations_preserve_set_semantics_and_leave_inputs_unchanged() {
        let left = BoltRefs::parse("[b, a]").unwrap();
        let right = BoltRefs::parse("[c, b]").unwrap();
        assert_eq!(left.combine(&right).emit(), "[a, b, c]");
        assert_eq!(left.divide(&right).emit(), "[a]");
        assert_eq!(left.filter(|slug| slug == "b").emit(), "[b]");
        assert_eq!(
            left.map(|slug| format!("{slug}-done")).unwrap().emit(),
            "[a-done, b-done]"
        );
        assert_eq!(left.fold_left(String::new(), |acc, slug| acc + slug), "ab");
        assert_eq!(left.at(0), Some("a"));
        assert_eq!(left.at(usize::MAX), None);
        assert_eq!(BoltRefs::empty().at(0), None);
        assert_eq!(left.emit(), "[a, b]");
        assert_eq!(right.emit(), "[b, c]");
    }

    #[test]
    fn removal_and_the_observation_faces_agree_with_the_set() {
        let mut refs = BoltRefs::parse("[b1-first, b2-second]").unwrap();
        assert!(refs.contains("b1-first"));
        assert!(!refs.contains("b9-absent"));
        assert_eq!(refs.len(), 2);
        assert!(!refs.is_empty());
        assert_eq!(
            refs.to_string(),
            refs.emit(),
            "Display は emit の綴りそのもの"
        );

        refs.remove_slug("b1-first").unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(
            refs.remove_slug("b1-first").unwrap_err(),
            BoltRefsError::MissingSlug("b1-first".to_string()),
            "不在 slug は無言 no-op にしない"
        );

        refs.remove_slug("b2-second").unwrap();
        assert!(refs.is_empty());
        assert_eq!(refs.emit(), "[empty list]");
    }

    #[test]
    fn empty_forms_parse_and_emit_the_literal() {
        assert_eq!(BoltRefs::parse("").unwrap().emit(), "[empty list]");
        assert_eq!(
            BoltRefs::parse("[empty list]").unwrap().emit(),
            "[empty list]"
        );
    }

    #[test]
    fn emit_is_sorted_regardless_of_input_order() {
        let refs = BoltRefs::parse("[zeta, alpha, mid]").unwrap();
        assert_eq!(refs.emit(), "[alpha, mid, zeta]");
    }

    #[test]
    fn append_and_remove_refuse_to_silently_no_op() {
        let mut refs = BoltRefs::parse("[a]").unwrap();
        assert_eq!(
            refs.append_slug("a"),
            Err(BoltRefsError::DuplicateSlug("a".into()))
        );
        assert_eq!(
            refs.remove_slug("b"),
            Err(BoltRefsError::MissingSlug("b".into()))
        );
        refs.append_slug("b").unwrap();
        refs.remove_slug("a").unwrap();
        assert_eq!(refs.emit(), "[b]");
    }

    #[test]
    fn malformed_lists_are_rejected() {
        assert!(BoltRefs::parse("a, b").is_err());
        assert!(BoltRefs::parse("[a, , b]").is_err());
        assert!(BoltRefs::parse("[a").is_err());
    }

    proptest! {
        #[test]
        fn union_obeys_monoid_and_set_laws(
            a in proptest::collection::btree_set("[a-z]{1,4}", 0..6),
            b in proptest::collection::btree_set("[a-z]{1,4}", 0..6),
            c in proptest::collection::btree_set("[a-z]{1,4}", 0..6),
        ) {
            let refs = |values: BTreeSet<String>| {
                if values.is_empty() {
                    BoltRefs::empty()
                } else {
                    BoltRefs::parse(&format!("[{}]", values.into_iter().collect::<Vec<_>>().join(", "))).expect("生成したslug集合")
                }
            };
            let (a, b, c) = (refs(a), refs(b), refs(c));
            prop_assert_eq!(a.combine(&b).combine(&c), a.combine(&b.combine(&c)));
            prop_assert_eq!(&a.combine(&BoltRefs::empty()), &a);
            prop_assert_eq!(&BoltRefs::empty().combine(&a), &a);
            prop_assert_eq!(&a.combine(&a), &a);
            prop_assert_eq!(a.combine(&b), b.combine(&a));
            prop_assert_eq!(a.divide(&a), BoltRefs::empty());
            prop_assert_eq!(a.divide(&BoltRefs::empty()), a);
        }

        /// emit → parse → emit の round-trip は不動点 (決定的直列化)。
        #[test]
        fn emit_parse_round_trip_is_a_fixed_point(
            slugs in proptest::collection::btree_set("[a-z][a-z0-9-]{0,8}", 0..6)
        ) {
            let mut refs = BoltRefs::default();
            for s in &slugs { refs.append_slug(s).unwrap(); }
            let emitted = refs.emit();
            let reparsed = BoltRefs::parse(&emitted).unwrap();
            prop_assert_eq!(reparsed.emit(), emitted);
        }
    }

    #[test]
    fn map_rejects_invalid_refs_and_deduplicates_collisions() {
        let refs = BoltRefs::parse("[a, b]").unwrap();
        for invalid in ["", "empty list", " a", "a ", "a,b", "[a]", "a\nb", "a\rb"] {
            assert_eq!(
                refs.map(|_| invalid.to_string()),
                Err(BoltRefsError::Malformed(invalid.to_string()))
            );
        }
        assert_eq!(refs.map(|_| "same".to_string()).unwrap().emit(), "[same]");
        let count = |acc, _: &str| acc + 1;
        assert_eq!(BoltRefs::empty().fold_left(7, count), 7);
        assert_eq!(refs.fold_left(7, count), 9);
        assert!(refs.filter(|_| false).is_empty());
        assert_eq!(refs.at(1), Some("b"));
        assert_eq!(refs.at(2), None);
        let mut calls = 0;
        let mut transform = |_: &str| {
            calls += 1;
            "same".to_string()
        };
        let mapped = BoltRefs::empty().map(&mut transform).unwrap();
        assert!(mapped.is_empty());
        assert_eq!(refs.map(&mut transform).unwrap().len(), 1);
        assert_eq!(calls, 2);
        assert_eq!(refs.emit(), "[a, b]");
    }
}
