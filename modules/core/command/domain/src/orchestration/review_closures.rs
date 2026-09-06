//! `ReviewClosures` — 現在の試行で閉じたレビュー依頼の列 (BR5.5)。

use core_infrastructure::collections::FirstClassCollection;

use crate::workflow_definition::ReviewPolicy;

use super::review_closure::ReviewClosure;

/// 判定が返って閉じた依頼の列 (記録順)。
///
/// [`ReviewAttempt`] の会計のうち「閉じた分」を持つ。終端の受領証があるかの判断
/// ([`ReviewClosures::has_terminal`]) はこの列が所有する — 呼び手が要素を取り出して自前で
/// 走査すると、受領証の会計という業務判断がコレクションの外へ漏れる
/// (`coding-rules/tell-dont-ask.md`)。
///
/// [`ReviewAttempt`]: super::ReviewAttempt
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReviewClosures {
    items: Vec<ReviewClosure>,
}

impl ReviewClosures {
    /// まだ 1 件も閉じていない試行の列。
    #[must_use]
    pub const fn empty() -> ReviewClosures {
        ReviewClosures { items: Vec::new() }
    }

    /// 保存された行から列を組み直す (**永続化境界からの再構成専用**)。
    ///
    /// 通常の構築は [`ReviewClosures::empty`] と [`ReviewClosures::record`] である。
    #[must_use]
    pub const fn new(items: Vec<ReviewClosure>) -> ReviewClosures {
        ReviewClosures { items }
    }

    /// 判定が返った依頼を 1 件記録する (記録順に積む)。
    pub fn record(&mut self, closure: ReviewClosure) {
        self.items.push(closure);
    }

    /// この列に**終端の受領証**があるか。
    ///
    /// # 非終端の NOT-READY は読み飛ばす (無効化しない)
    ///
    /// upstream で非終端 NOT-READY が受領証を無効化するのは、成果物 fingerprint が使える
    /// ときだけである (`aidlc-lib.ts:5218`)。本 build は fingerprint を繰延しているので、
    /// 非終端の判定は単に終端ではないという扱いになる。
    #[must_use]
    pub fn has_terminal(&self, policy: &ReviewPolicy) -> bool {
        self.fold_left(false, |found, closure| {
            found || policy.is_terminal(closure.verdict(), closure.iteration())
        })
    }

    /// 閉じた依頼の件数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }

    /// 1 件も閉じていないか。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 記録順の添字参照。範囲外は `None` (panic しない)。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<ReviewClosure> {
        self.items.get(index).copied()
    }

    /// 記録順に左から畳み込む。空なら初期値を返す。
    pub fn fold_left<A>(&self, initial: A, mut fold: impl FnMut(A, ReviewClosure) -> A) -> A {
        self.items
            .iter()
            .fold(initial, |acc, closure| fold(acc, *closure))
    }

    /// 条件に一致する受領証を記録順のまま残す。結果は空になり得る。
    #[must_use]
    pub fn filter(&self, mut predicate: impl FnMut(ReviewClosure) -> bool) -> ReviewClosures {
        ReviewClosures {
            items: self
                .items
                .iter()
                .filter(|closure| predicate(**closure))
                .copied()
                .collect(),
        }
    }
}

impl FirstClassCollection for ReviewClosures {
    type Item<'a> = ReviewClosure;
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
    use super::ReviewClosures;
    use crate::orchestration::{ReviewClosure, ReviewVerdict};
    use crate::workflow_definition::{ReviewCapValue, ReviewPolicy};
    use core_infrastructure::collections::FirstClassCollection;

    fn policy(effective: ReviewCapValue) -> ReviewPolicy {
        ReviewPolicy::new("aidlc-quality-agent", effective, 2, false)
    }

    fn two_closures() -> ReviewClosures {
        ReviewClosures::new(vec![
            ReviewClosure::new(1, ReviewVerdict::NotReady),
            ReviewClosure::new(2, ReviewVerdict::Ready),
        ])
    }

    #[test]
    fn a_fresh_attempt_has_closed_nothing() {
        let empty = ReviewClosures::empty();
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
        assert_eq!(empty.at(0), None);
        assert_eq!(ReviewClosures::default(), empty);
        assert!(!empty.has_terminal(&policy(ReviewCapValue::Adversarial)));
    }

    #[test]
    fn recording_appends_in_the_order_the_verdicts_landed() {
        let mut closures = ReviewClosures::empty();
        closures.record(ReviewClosure::new(1, ReviewVerdict::NotReady));
        closures.record(ReviewClosure::new(2, ReviewVerdict::Ready));
        assert_eq!(closures, two_closures());
        assert_eq!(
            closures.at(0),
            Some(ReviewClosure::new(1, ReviewVerdict::NotReady))
        );
        assert_eq!(
            closures.at(1),
            Some(ReviewClosure::new(2, ReviewVerdict::Ready))
        );
    }

    #[test]
    fn a_position_past_the_end_is_none_and_never_panics() {
        let closures = two_closures();
        assert_eq!(closures.at(2), None);
        assert_eq!(closures.at(usize::MAX), None);
    }

    #[test]
    fn a_non_terminal_not_ready_is_read_past_rather_than_treated_as_a_receipt() {
        let mut closures = ReviewClosures::empty();
        closures.record(ReviewClosure::new(1, ReviewVerdict::NotReady));
        assert!(
            !closures.has_terminal(&policy(ReviewCapValue::Adversarial)),
            "adversarial の 1 回目 NOT-READY は終端ではない"
        );
        closures.record(ReviewClosure::new(2, ReviewVerdict::NotReady));
        assert!(
            closures.has_terminal(&policy(ReviewCapValue::Adversarial)),
            "反復上限に達した NOT-READY は終端"
        );
    }

    #[test]
    fn a_ready_verdict_is_terminal_and_a_disabled_policy_never_is() {
        let closures = two_closures();
        assert!(closures.has_terminal(&policy(ReviewCapValue::Adversarial)));
        assert!(closures.has_terminal(&policy(ReviewCapValue::Advisory)));
        assert!(
            !closures.has_terminal(&policy(ReviewCapValue::None)),
            "レビュアーを呼ばない実行に終端の受領証は生まれない"
        );
    }

    #[test]
    fn folding_and_filtering_walk_the_recording_order() {
        let closures = two_closures();
        assert_eq!(
            closures.fold_left(Vec::new(), |mut acc, closure| {
                acc.push(closure.iteration());
                acc
            }),
            [1, 2]
        );
        let ready = closures.filter(|closure| closure.verdict() == ReviewVerdict::Ready);
        assert_eq!(
            ready,
            ReviewClosures::new(vec![ReviewClosure::new(2, ReviewVerdict::Ready)])
        );
        assert!(closures.filter(|_| false).is_empty());
        assert_eq!(closures.len(), 2, "元の列は変わらない");
    }

    #[test]
    fn the_shared_traversal_contract_sees_the_same_list() {
        let closures = two_closures();
        assert_eq!(FirstClassCollection::len(&closures), 2);
        assert!(!FirstClassCollection::is_empty(&closures));
        assert_eq!(
            FirstClassCollection::at(&closures, 0),
            Some(ReviewClosure::new(1, ReviewVerdict::NotReady))
        );
        assert_eq!(FirstClassCollection::at(&closures, 2), None);
        assert_eq!(
            FirstClassCollection::fold_left(&closures, 0, |count, _| count + 1),
            2
        );
        assert_eq!(FirstClassCollection::filter(&closures, |_| true), closures);
    }
}
