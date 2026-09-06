//! `TransitionSteps` — 報告の適用が踏む遷移サブコマンドの列 (BR5.5)。

use core_infrastructure::collections::FirstClassCollection;

use super::transition_step::TransitionStep;
use super::transition_steps_error::TransitionStepsError;

/// `ReportDecision::Commit` が名指す遷移サブコマンドの列 (`report_dispatch` が決めた順)。
///
/// 段は重複しない — 1 回の報告適用で同じ遷移を 2 度踏むことはないので、重複は静かに畳まず
/// 構築時に拒否する。列の**形**は名前付きクエリ ([`TransitionSteps::is_single`] /
/// [`TransitionSteps::is_pair`]) で問う: 適用側が生のスライスを受け取って自前で分岐すると、
/// 段の並びという業務判断が集約の外へ漏れる (`coding-rules/tell-dont-ask.md`)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TransitionSteps {
    items: Vec<TransitionStep>,
}

impl TransitionSteps {
    /// 遷移順の段から列を組む (**DTO とディスパッチの唯一の構築経路**)。
    ///
    /// # Errors
    ///
    /// 同じ段が 2 回以上現れる場合は [`TransitionStepsError::Duplicate`]。
    pub fn new(items: Vec<TransitionStep>) -> Result<TransitionSteps, TransitionStepsError> {
        let mut seen: Vec<TransitionStep> = Vec::with_capacity(items.len());
        for step in &items {
            if seen.contains(step) {
                return Err(TransitionStepsError::Duplicate { step: *step });
            }
            seen.push(*step);
        }
        Ok(TransitionSteps { items })
    }

    /// 段 1 つだけの列 (重複し得ないので検査を要さない)。
    #[must_use]
    pub fn single(step: TransitionStep) -> TransitionSteps {
        TransitionSteps { items: vec![step] }
    }

    /// 復旧の 2 段 — ゲートを開き直してから承認する。
    ///
    /// upstream の `sequence` に現れる**唯一の 2 段**であり (`handleReport` 段 13 の
    /// 「ゲート付き × `[-]`」の行)、2 つの段は異なるので重複は構造的に起こり得ない。
    /// [`TransitionSteps::new`] の `Result` を呼出側で開かずに済むよう、この列だけは
    /// 名前付きの全域構築子を持つ (プロダクトコードで `unwrap` を使わないため)。
    #[must_use]
    pub fn recovered_approval() -> TransitionSteps {
        TransitionSteps {
            items: vec![TransitionStep::GateStartRecovered, TransitionStep::Approve],
        }
    }

    /// その段を含むか。
    #[must_use]
    pub fn contains(&self, step: TransitionStep) -> bool {
        self.items.contains(&step)
    }

    /// その段 1 つだけの列か。
    #[must_use]
    pub fn is_single(&self, step: TransitionStep) -> bool {
        self.items.len() == 1 && self.at(0) == Some(step)
    }

    /// その 2 段がこの順に並んだ列か。
    #[must_use]
    pub fn is_pair(&self, first: TransitionStep, second: TransitionStep) -> bool {
        self.items.len() == 2 && self.at(0) == Some(first) && self.at(1) == Some(second)
    }

    /// 段の数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }

    /// 段が 1 つも無いか。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 遷移順の添字参照。範囲外は `None` (panic しない)。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<TransitionStep> {
        self.items.get(index).copied()
    }

    /// 遷移順に左から畳み込む。空なら初期値を返す。
    pub fn fold_left<A>(&self, initial: A, mut fold: impl FnMut(A, TransitionStep) -> A) -> A {
        self.items
            .iter()
            .fold(initial, |acc, step| fold(acc, *step))
    }

    /// 条件に一致する段を遷移順のまま残す。重複が増えないので不変条件は保たれる。
    #[must_use]
    pub fn filter(&self, mut predicate: impl FnMut(TransitionStep) -> bool) -> TransitionSteps {
        TransitionSteps {
            items: self
                .items
                .iter()
                .filter(|step| predicate(**step))
                .copied()
                .collect(),
        }
    }
}

impl FirstClassCollection for TransitionSteps {
    type Item<'a> = TransitionStep;
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
    use super::TransitionSteps;
    use crate::orchestration::{TransitionStep, TransitionStepsError};
    use core_infrastructure::collections::FirstClassCollection;

    fn recovered_approve() -> TransitionSteps {
        TransitionSteps::new(vec![
            TransitionStep::GateStartRecovered,
            TransitionStep::Approve,
        ])
        .unwrap()
    }

    #[test]
    fn an_empty_sequence_is_accepted_and_carries_no_step() {
        let empty = TransitionSteps::new(Vec::new()).unwrap();
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
        assert_eq!(empty.at(0), None);
        assert!(!empty.contains(TransitionStep::Approve));
    }

    #[test]
    fn the_sequence_keeps_the_dispatch_order() {
        let steps = recovered_approve();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps.at(0), Some(TransitionStep::GateStartRecovered));
        assert_eq!(steps.at(1), Some(TransitionStep::Approve));
        assert_eq!(steps.at(2), None);
        assert_eq!(steps.at(usize::MAX), None);
    }

    /// 復旧の 2 段は名前で組める (重複が構造的に起き得ないので `Result` を返さない)。
    #[test]
    fn the_recovery_sequence_opens_the_gate_again_before_approving() {
        let steps = TransitionSteps::recovered_approval();
        assert!(steps.is_pair(TransitionStep::GateStartRecovered, TransitionStep::Approve));
        assert_eq!(steps.len(), 2);
        assert_eq!(steps, recovered_approve());
    }

    #[test]
    fn a_repeated_step_is_rejected_instead_of_being_folded_away() {
        assert_eq!(
            TransitionSteps::new(vec![TransitionStep::Approve, TransitionStep::Approve])
                .unwrap_err(),
            TransitionStepsError::Duplicate {
                step: TransitionStep::Approve
            }
        );
    }

    #[test]
    fn the_two_gate_start_variants_are_distinct_steps() {
        let steps = TransitionSteps::new(vec![
            TransitionStep::GateStart,
            TransitionStep::GateStartRecovered,
        ])
        .unwrap();
        assert_eq!(steps.len(), 2, "同じ subcommand 綴りでも別の段");
    }

    #[test]
    fn a_single_step_sequence_names_its_shape() {
        let only = TransitionSteps::single(TransitionStep::Skip);
        assert_eq!(only.len(), 1);
        assert!(only.contains(TransitionStep::Skip));
        assert!(only.is_single(TransitionStep::Skip));
        assert!(!only.is_single(TransitionStep::Approve));
        assert!(!only.is_pair(TransitionStep::Skip, TransitionStep::Approve));
    }

    #[test]
    fn a_two_step_sequence_names_its_shape_in_order() {
        let steps = recovered_approve();
        assert!(steps.is_pair(TransitionStep::GateStartRecovered, TransitionStep::Approve));
        assert!(
            !steps.is_pair(TransitionStep::Approve, TransitionStep::GateStartRecovered),
            "順序が違えば別の形"
        );
        assert!(!steps.is_single(TransitionStep::Approve));
        assert!(steps.contains(TransitionStep::Approve));
        assert!(!steps.contains(TransitionStep::Reject));
    }

    #[test]
    fn folding_and_filtering_walk_the_dispatch_order() {
        let steps = recovered_approve();
        assert_eq!(
            steps.fold_left(String::new(), |acc, step| acc + step.subcommand() + "|"),
            "gate-start|approve|"
        );
        let kept = steps.filter(|step| step == TransitionStep::Approve);
        assert_eq!(kept, TransitionSteps::single(TransitionStep::Approve));
        assert!(steps.filter(|_| false).is_empty());
        assert_eq!(steps.len(), 2, "元の列は変わらない");
    }

    #[test]
    fn the_shared_traversal_contract_sees_the_same_sequence() {
        let steps = recovered_approve();
        assert_eq!(FirstClassCollection::len(&steps), 2);
        assert!(!FirstClassCollection::is_empty(&steps));
        assert_eq!(
            FirstClassCollection::at(&steps, 1),
            Some(TransitionStep::Approve)
        );
        assert_eq!(FirstClassCollection::at(&steps, 2), None);
        assert_eq!(
            FirstClassCollection::fold_left(&steps, 0, |count, _| count + 1),
            2
        );
        assert_eq!(FirstClassCollection::filter(&steps, |_| true), steps);
    }
}
