//! `ReviewerScopeCandidates` — 呼出し 1 件が名指した候補の列。
use super::ReviewerScopeCandidate;
use core_infrastructure::collections::FirstClassCollection;

/// 工具の入力から掲載順に集めた候補 (upstream `candidateStrings` の戻り値)。
///
/// **順序に意味がある** — upstream は最初に拒否へ当たった候補で判定を打ち切り、その綴りを
/// 監査と拒否文言へ載せる。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReviewerScopeCandidates {
    items: Vec<ReviewerScopeCandidate>,
}
impl ReviewerScopeCandidates {
    /// 候補の列を固定する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(items: Vec<ReviewerScopeCandidate>) -> ReviewerScopeCandidates {
        ReviewerScopeCandidates { items }
    }
    /// 候補の件数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }
    /// 候補が 1 つも無いか。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    /// 掲載順の添字参照。範囲外は `None`。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&ReviewerScopeCandidate> {
        self.items.get(index)
    }
}
impl FirstClassCollection for ReviewerScopeCandidates {
    type Item<'a> = &'a ReviewerScopeCandidate;
    type Filtered = Self;
    fn len(&self) -> usize {
        ReviewerScopeCandidates::len(self)
    }
    fn at(&self, index: usize) -> Option<&ReviewerScopeCandidate> {
        ReviewerScopeCandidates::at(self, index)
    }
    fn fold_left<'a, A>(
        &'a self,
        initial: A,
        fold: impl FnMut(A, &'a ReviewerScopeCandidate) -> A,
    ) -> A {
        self.items.iter().fold(initial, fold)
    }
    fn filter(&self, mut predicate: impl FnMut(&ReviewerScopeCandidate) -> bool) -> Self {
        ReviewerScopeCandidates::new(
            self.items
                .iter()
                .filter(|candidate| predicate(candidate))
                .cloned()
                .collect(),
        )
    }
}
