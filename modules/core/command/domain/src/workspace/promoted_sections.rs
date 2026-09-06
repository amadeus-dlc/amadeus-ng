//! `PromotedSections` — 昇格が team.md へ書き替える節の列 (BR5.5)。

use std::collections::BTreeSet;

use core_infrastructure::collections::{Collection, FirstClassCollection};

use super::promoted_section::PromotedSection;
use super::promoted_sections_error::PromotedSectionsError;

/// 昇格が置き換える節の列 (書き込み順)。
///
/// 見出しは一意である — 同じ節を 2 回書けば後勝ちで本文が消え、監査行の
/// `Sections Written` からどちらが書かれたか追えなくなるので、構築時に拒否する。
/// 順序は upstream の `sectionsWritten` の綴り順そのものなので並べ替えない。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PromotedSections {
    items: Vec<PromotedSection>,
}

impl PromotedSections {
    /// 書き込み順の節から列を組む (**この型の唯一の構築経路**)。
    ///
    /// # Errors
    ///
    /// 同じ見出しが 2 回以上現れる場合は [`PromotedSectionsError::DuplicateHeading`]。
    pub fn new(items: Vec<PromotedSection>) -> Result<PromotedSections, PromotedSectionsError> {
        let mut seen = BTreeSet::new();
        for section in &items {
            if !seen.insert(section.heading()) {
                return Err(PromotedSectionsError::DuplicateHeading {
                    heading: section.heading().to_string(),
                });
            }
        }
        Ok(PromotedSections { items })
    }

    /// 書き込み順に並んだ見出し名の列 (`## ` を含まない)。
    #[must_use]
    pub fn headings(&self) -> Collection<String> {
        Collection::new(self.fold_left(Vec::new(), |mut headings, section| {
            headings.push(section.heading().to_string());
            headings
        }))
    }

    /// 節の件数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }

    /// 書き替える節が 1 つも無いか。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 書き込み順の添字参照。範囲外は `None` (panic しない)。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&PromotedSection> {
        self.items.get(index)
    }

    /// 書き込み順に左から畳み込む。空なら初期値を返す。
    pub fn fold_left<'a, A>(
        &'a self,
        initial: A,
        fold: impl FnMut(A, &'a PromotedSection) -> A,
    ) -> A {
        self.items.iter().fold(initial, fold)
    }

    /// 条件に一致する節を書き込み順のまま残す。見出しは一意のままなので不変条件は保たれる。
    #[must_use]
    pub fn filter(&self, mut predicate: impl FnMut(&PromotedSection) -> bool) -> PromotedSections {
        PromotedSections {
            items: self
                .items
                .iter()
                .filter(|section| predicate(section))
                .cloned()
                .collect(),
        }
    }
}

impl FirstClassCollection for PromotedSections {
    type Item<'a> = &'a PromotedSection;
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
    use super::PromotedSections;
    use crate::workspace::{PromotedSection, PromotedSectionsError};
    use core_infrastructure::collections::{Collection, FirstClassCollection};

    fn sections() -> PromotedSections {
        PromotedSections::new(vec![
            PromotedSection::new("Way of Working", "trunk-based.\n"),
            PromotedSection::new("Testing Posture", "tdd.\n"),
        ])
        .unwrap()
    }

    #[test]
    fn an_empty_promotion_writes_no_section() {
        let empty = PromotedSections::new(Vec::new()).unwrap();
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
        assert_eq!(empty.at(0), None);
        assert_eq!(empty.headings(), Collection::empty());
    }

    #[test]
    fn the_list_keeps_the_order_the_promotion_writes_them_in() {
        let sections = sections();
        assert_eq!(sections.len(), 2);
        assert_eq!(
            sections.at(0).map(PromotedSection::heading),
            Some("Way of Working")
        );
        assert_eq!(
            sections.at(1).map(PromotedSection::heading),
            Some("Testing Posture")
        );
    }

    #[test]
    fn a_repeated_heading_is_rejected_instead_of_being_written_twice() {
        assert_eq!(
            PromotedSections::new(vec![
                PromotedSection::new("Way of Working", "first"),
                PromotedSection::new("Deployment", "second"),
                PromotedSection::new("Way of Working", "third"),
            ])
            .unwrap_err(),
            PromotedSectionsError::DuplicateHeading {
                heading: "Way of Working".to_string(),
            }
        );
    }

    #[test]
    fn a_position_past_the_end_is_none_and_never_panics() {
        let sections = sections();
        assert_eq!(sections.at(2), None);
        assert_eq!(sections.at(usize::MAX), None);
    }

    #[test]
    fn the_headings_are_read_in_the_written_order() {
        assert_eq!(
            sections().headings(),
            Collection::new(vec![
                "Way of Working".to_string(),
                "Testing Posture".to_string(),
            ])
        );
    }

    #[test]
    fn folding_and_filtering_walk_the_written_order() {
        let sections = sections();
        assert_eq!(
            sections.fold_left(String::new(), |acc, section| acc + section.body()),
            "trunk-based.\ntdd.\n"
        );
        let kept = sections.filter(|section| section.heading() == "Testing Posture");
        assert_eq!(kept.len(), 1);
        assert_eq!(kept.at(0).map(PromotedSection::body), Some("tdd.\n"));
        assert!(sections.filter(|_| false).is_empty());
        assert_eq!(sections.len(), 2, "元の列は変わらない");
    }

    #[test]
    fn the_shared_traversal_contract_sees_the_same_list() {
        let sections = sections();
        assert_eq!(FirstClassCollection::len(&sections), 2);
        assert!(!FirstClassCollection::is_empty(&sections));
        assert_eq!(
            FirstClassCollection::at(&sections, 0).map(PromotedSection::heading),
            Some("Way of Working")
        );
        assert_eq!(FirstClassCollection::at(&sections, 2), None);
        assert_eq!(
            FirstClassCollection::fold_left(&sections, 0, |count, _| count + 1),
            2
        );
        assert_eq!(FirstClassCollection::filter(&sections, |_| true), sections);
    }
}
