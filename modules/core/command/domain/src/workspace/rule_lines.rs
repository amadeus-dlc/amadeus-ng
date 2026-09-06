//! `RuleLines` — 昇格が memory 層へ書き足す規則行の列 (BR5.5)。

use core_infrastructure::collections::FirstClassCollection;

/// `PracticesPromotion` / `PracticesAffirmed` が運ぶ `## Mandated` / `## Forbidden` の行。
///
/// **素通しの列**である — 順序も重複も入力のまま保つ (NFR4.4)。行は upstream の綴りその
/// ままで memory ファイルへ書き足されるので、ドメインは並べ替えも重複除去も行わない。集合
/// ではないので `combine` / `divide` は持たない
/// (`coding-rules/first-class-collections.md` — 順序付き列を便宜的な集合として扱わない)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuleLines {
    items: Vec<String>,
}

impl RuleLines {
    /// 1 行も書き足さない昇格の列。
    #[must_use]
    pub const fn empty() -> RuleLines {
        RuleLines { items: Vec::new() }
    }

    /// 与えられた順序と重複のまま列にする (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(items: Vec<String>) -> RuleLines {
        RuleLines { items }
    }

    /// 規則行の件数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }

    /// 規則行が 1 つも無いか。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 格納順の添字参照。範囲外は `None` (panic しない)。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&str> {
        self.items.get(index).map(String::as_str)
    }

    /// 格納順に左から畳み込む。空なら初期値を返す。
    pub fn fold_left<'a, A>(&'a self, initial: A, mut fold: impl FnMut(A, &'a str) -> A) -> A {
        self.items
            .iter()
            .fold(initial, |acc, line| fold(acc, line.as_str()))
    }

    /// 条件に一致する行を格納順のまま残す。結果は空になり得る。
    #[must_use]
    pub fn filter(&self, mut predicate: impl FnMut(&str) -> bool) -> RuleLines {
        RuleLines::new(
            self.items
                .iter()
                .filter(|line| predicate(line))
                .cloned()
                .collect(),
        )
    }
}

impl FirstClassCollection for RuleLines {
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
    use super::RuleLines;
    use core_infrastructure::collections::FirstClassCollection;

    fn lines() -> RuleLines {
        RuleLines::new(vec![
            "ALWAYS write tests first".to_string(),
            "NEVER open two pull requests".to_string(),
            "ALWAYS write tests first".to_string(),
        ])
    }

    #[test]
    fn the_empty_list_carries_no_rule() {
        let empty = RuleLines::empty();
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
        assert_eq!(empty.at(0), None);
        assert_eq!(empty.fold_left(0, |count, _| count + 1), 0);
    }

    #[test]
    fn the_list_passes_the_rules_through_in_order_and_keeps_duplicates() {
        let lines = lines();
        assert_eq!(lines.len(), 3);
        assert!(!lines.is_empty());
        assert_eq!(lines.at(0), Some("ALWAYS write tests first"));
        assert_eq!(lines.at(1), Some("NEVER open two pull requests"));
        assert_eq!(lines.at(2), Some("ALWAYS write tests first"));
    }

    #[test]
    fn a_position_past_the_end_is_none_and_never_panics() {
        let lines = lines();
        assert_eq!(lines.at(3), None);
        assert_eq!(lines.at(usize::MAX), None);
    }

    #[test]
    fn folding_walks_the_list_from_the_left_in_the_stored_order() {
        assert_eq!(
            lines().fold_left(String::new(), |acc, line| acc
                + line.split_whitespace().next().unwrap_or_default()
                + "|"),
            "ALWAYS|NEVER|ALWAYS|"
        );
    }

    #[test]
    fn filtering_keeps_the_stored_order_and_can_empty_the_list() {
        let lines = lines();
        let mandated = lines.filter(|line| line.starts_with("ALWAYS"));
        assert_eq!(mandated.len(), 2);
        assert_eq!(mandated.at(0), Some("ALWAYS write tests first"));
        assert!(lines.filter(|_| false).is_empty());
        assert_eq!(lines.len(), 3, "元の列は変わらない");
    }

    #[test]
    fn the_default_list_is_the_empty_one() {
        assert_eq!(RuleLines::default(), RuleLines::empty());
    }

    #[test]
    fn the_shared_traversal_contract_sees_the_same_list() {
        let lines = lines();
        assert_eq!(FirstClassCollection::len(&lines), 3);
        assert!(!FirstClassCollection::is_empty(&lines));
        assert_eq!(
            FirstClassCollection::at(&lines, 1),
            Some("NEVER open two pull requests")
        );
        assert_eq!(FirstClassCollection::at(&lines, 3), None);
        assert_eq!(
            FirstClassCollection::fold_left(&lines, 0, |count, _| count + 1),
            3
        );
        assert_eq!(FirstClassCollection::filter(&lines, |_| true), lines);
    }
}
