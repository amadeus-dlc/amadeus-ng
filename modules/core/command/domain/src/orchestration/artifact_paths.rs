//! `ArtifactPaths` — ゲート開放が運ぶ成果物パスの列 (BR5.5)。

use core_infrastructure::collections::FirstClassCollection;

/// `GateOpened` が運ぶ成果物パスの列。
///
/// **素通しの列**である — 順序も重複も入力のまま保つ (NFR4.4)。パスはゲートを開いた側が
/// 名指した綴りそのものであり、ドメインは並べ替えも重複除去も正規化もしない。集合ではない
/// ので `combine` / `divide` は持たない (`coding-rules/first-class-collections.md` — 順序付き
/// 列を便宜的な集合として扱わない)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ArtifactPaths {
    items: Vec<String>,
}

impl ArtifactPaths {
    /// 成果物を 1 つも伴わないゲート開放の列。
    #[must_use]
    pub const fn empty() -> ArtifactPaths {
        ArtifactPaths { items: Vec::new() }
    }

    /// 与えられた順序と重複のまま列にする (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(items: Vec<String>) -> ArtifactPaths {
        ArtifactPaths { items }
    }

    /// 成果物の件数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }

    /// 成果物が 1 つも無いか。
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
            .fold(initial, |acc, path| fold(acc, path.as_str()))
    }

    /// 条件に一致するパスを格納順のまま残す。結果は空になり得る。
    #[must_use]
    pub fn filter(&self, mut predicate: impl FnMut(&str) -> bool) -> ArtifactPaths {
        ArtifactPaths::new(
            self.items
                .iter()
                .filter(|path| predicate(path))
                .cloned()
                .collect(),
        )
    }
}

impl FirstClassCollection for ArtifactPaths {
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
    use super::ArtifactPaths;
    use core_infrastructure::collections::FirstClassCollection;

    fn paths() -> ArtifactPaths {
        ArtifactPaths::new(vec![
            "requirements.md".to_string(),
            "design.md".to_string(),
            "requirements.md".to_string(),
        ])
    }

    #[test]
    fn the_empty_list_carries_no_path() {
        let empty = ArtifactPaths::empty();
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
        assert_eq!(empty.at(0), None);
        assert_eq!(empty.fold_left(0, |count, _| count + 1), 0);
    }

    #[test]
    fn the_list_passes_the_paths_through_in_order_and_keeps_duplicates() {
        let paths = paths();
        assert_eq!(paths.len(), 3);
        assert!(!paths.is_empty());
        assert_eq!(paths.at(0), Some("requirements.md"));
        assert_eq!(paths.at(1), Some("design.md"));
        assert_eq!(paths.at(2), Some("requirements.md"));
    }

    #[test]
    fn a_position_past_the_end_is_none_and_never_panics() {
        let paths = paths();
        assert_eq!(paths.at(3), None);
        assert_eq!(paths.at(usize::MAX), None);
    }

    #[test]
    fn folding_walks_the_list_from_the_left_in_the_stored_order() {
        assert_eq!(
            paths().fold_left(String::new(), |acc, path| acc + path + "|"),
            "requirements.md|design.md|requirements.md|"
        );
    }

    #[test]
    fn filtering_keeps_the_stored_order_and_can_empty_the_list() {
        let paths = paths();
        let kept = paths.filter(|path| path == "requirements.md");
        assert_eq!(
            kept,
            ArtifactPaths::new(vec![
                "requirements.md".to_string(),
                "requirements.md".to_string(),
            ])
        );
        assert!(paths.filter(|_| false).is_empty());
        assert_eq!(paths.len(), 3, "元の列は変わらない");
    }

    #[test]
    fn the_default_list_is_the_empty_one() {
        assert_eq!(ArtifactPaths::default(), ArtifactPaths::empty());
    }

    #[test]
    fn the_shared_traversal_contract_sees_the_same_list() {
        let paths = paths();
        assert_eq!(FirstClassCollection::len(&paths), 3);
        assert!(!FirstClassCollection::is_empty(&paths));
        assert_eq!(FirstClassCollection::at(&paths, 1), Some("design.md"));
        assert_eq!(FirstClassCollection::at(&paths, 3), None);
        assert_eq!(
            FirstClassCollection::fold_left(&paths, 0, |count, _| count + 1),
            3
        );
        assert_eq!(FirstClassCollection::filter(&paths, |_| true), paths);
    }
}
