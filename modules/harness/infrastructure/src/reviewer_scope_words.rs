//! reviewer-scope の字句規則で読んだ、実行位置 1 つ分の語の列。
use core_infrastructure::collections::FirstClassCollection;

/// シェルコマンドの 1 実行位置に並ぶ語 (upstream `aidlc-reviewer-scope.ts` の
/// `ShellWord[]` に対応する)。
///
/// **同じクレートの [`crate::ShellWords`] とは別の上流関数の移植であり、字句規則が違う。**
/// 混ぜて使わないこと。主な差は次のとおり。
///
/// | 観点 | [`crate::ShellWords`] (`aidlc-state-transition-guard` 系) | 本型 (`reviewer-scope`) |
/// | --- | --- | --- |
/// | `;` `\|` `&` `(` `)` | 語の一部に残す / 区間を切る | **語を切り、区切りにする** |
/// | `<` `>` | 同上 | **普通の文字**。`cat>x` は 1 語 |
/// | `$'…'` | ANSI-C 引用として扱う | `$` は普通の文字、`'` が引用を開く |
/// | 空の引用語 `""` | 語として残す | **語にしない** |
/// | 行継続 `\`+改行 | 継続として畳む | 単なる逃がし (改行が語に入る) |
///
/// upstream は語ごとに `quoted` を立てるが**どこからも読んでいない**ので持たない。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReviewerScopeWords {
    items: Vec<String>,
}
impl ReviewerScopeWords {
    /// 語の列を固定する (**この型の唯一の構築経路**)。
    pub(crate) const fn new(items: Vec<String>) -> ReviewerScopeWords {
        ReviewerScopeWords { items }
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
    pub fn at(&self, index: usize) -> Option<&str> {
        self.items.get(index).map(String::as_str)
    }
}
impl FirstClassCollection for ReviewerScopeWords {
    type Item<'a> = &'a str;
    type Filtered = Self;
    fn len(&self) -> usize {
        ReviewerScopeWords::len(self)
    }
    fn at(&self, index: usize) -> Option<&str> {
        ReviewerScopeWords::at(self, index)
    }
    fn fold_left<'a, A>(&'a self, initial: A, mut fold: impl FnMut(A, &'a str) -> A) -> A {
        self.items
            .iter()
            .fold(initial, |acc, word| fold(acc, word.as_str()))
    }
    fn filter(&self, mut predicate: impl FnMut(&str) -> bool) -> Self {
        ReviewerScopeWords::new(
            self.items
                .iter()
                .filter(|word| predicate(word))
                .cloned()
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::ReviewerScopeWords;
    use core_infrastructure::collections::FirstClassCollection;

    fn words(items: &[&str]) -> ReviewerScopeWords {
        ReviewerScopeWords::new(items.iter().map(|item| (*item).to_string()).collect())
    }

    #[test]
    fn an_empty_word_list_reports_itself_as_empty() {
        let empty = words(&[]);
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        assert_eq!(empty.at(0), None);
        assert!(ReviewerScopeWords::default().is_empty());
        assert!(!words(&["cat"]).is_empty());
    }

    #[test]
    fn the_collection_exposes_count_index_fold_and_filter() {
        let words = words(&["cat", "a.md", "b.txt"]);
        assert_eq!(FirstClassCollection::len(&words), 3);
        assert_eq!(FirstClassCollection::at(&words, 2), Some("b.txt"));
        assert_eq!(FirstClassCollection::at(&words, 3), None);
        let total = words.fold_left(0, |acc, word| acc + word.len());
        assert_eq!(total, 3 + 4 + 5);
        let markdown = words.filter(|word| word.ends_with(".md"));
        assert_eq!(markdown.len(), 1);
        assert_eq!(markdown.at(0), Some("a.md"));
    }
}
