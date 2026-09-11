//! `LearningObservations` — 選択された学びと両側の実測の列。

use core_infrastructure::collections::FirstClassCollection;

use super::captured_learning::CapturedLearning;
use super::captured_learnings::CapturedLearnings;
use super::learning_observation::LearningObservation;

/// persist へ渡された選択の列（選択順）。
///
/// **素通しの列**である — 並べ替えない。同じ本文が 2 度現れたときの扱いは
/// [`CapturedLearnings`] を導く [`LearningObservations::captured`] が決める。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearningObservations {
    items: Vec<LearningObservation>,
}

impl Default for LearningObservations {
    fn default() -> Self {
        Self::of_items(Default::default())
    }
}

impl LearningObservations {
    // 検査済みの列とその部分列は、この構築口で全状態を初期化する。
    const fn of_items(items: Vec<LearningObservation>) -> Self {
        Self { items }
    }

    /// 何も選ばれなかった回。
    #[must_use]
    pub const fn empty() -> LearningObservations {
        LearningObservations::of_items(Vec::new())
    }

    /// 選択順のまま列にする（**この型の唯一の構築経路**）。
    #[must_use]
    pub const fn new(items: Vec<LearningObservation>) -> LearningObservations {
        LearningObservations::of_items(items)
    }

    /// 選択の件数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }

    /// 選択が 1 件も無いか。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 選択順の添字参照。範囲外は `None`（panic しない）。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&LearningObservation> {
        self.items.get(index)
    }

    /// 選択順に左から畳み込む。空なら初期値を返す。
    pub fn fold_left<'a, A>(
        &'a self,
        initial: A,
        fold: impl FnMut(A, &'a LearningObservation) -> A,
    ) -> A {
        self.items.iter().fold(initial, fold)
    }

    /// 条件に一致する選択を選択順のまま残す。結果は空になり得る。
    #[must_use]
    pub fn filter(&self, mut predicate: impl FnMut(&LearningObservation) -> bool) -> Self {
        LearningObservations::new(
            self.items
                .iter()
                .filter(|observation| predicate(observation))
                .cloned()
                .collect(),
        )
    }

    /// この回に書く学びを決める。
    ///
    /// 同じ本文が 2 度現れたら最初の 1 件だけを残し（本家の `batchRuleHashes`）、両側が
    /// 既に在るものは落とす。
    #[must_use]
    pub fn captured(&self) -> CapturedLearnings {
        let (captured, _) = self.fold_left(
            (Vec::new(), std::collections::BTreeSet::new()),
            |(mut captured, mut seen): (Vec<CapturedLearning>, std::collections::BTreeSet<_>),
             observation| {
                let Some(disposition) = observation.disposition() else {
                    seen.insert(observation.learning().content_hash().clone());
                    return (captured, seen);
                };
                if !seen.insert(observation.learning().content_hash().clone()) {
                    return (captured, seen);
                }
                captured.push(CapturedLearning::new(
                    observation.learning().clone(),
                    disposition,
                ));
                (captured, seen)
            },
        );
        CapturedLearnings::new(captured)
    }
}

impl FirstClassCollection for LearningObservations {
    type Item<'a> = &'a LearningObservation;
    type Filtered = Self;
    fn len(&self) -> usize {
        Self::len(self)
    }
    fn at(&self, index: usize) -> Option<&LearningObservation> {
        Self::at(self, index)
    }
    fn fold_left<'a, A>(
        &'a self,
        initial: A,
        fold: impl FnMut(A, &'a LearningObservation) -> A,
    ) -> A {
        Self::fold_left(self, initial, fold)
    }
    fn filter(&self, predicate: impl FnMut(&LearningObservation) -> bool) -> Self {
        Self::filter(self, predicate)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::LearningObservations;
    use crate::orchestration::{
        Learning, LearningCandidateId, LearningDisposition, LearningObservation, LearningScope,
        LearningSource, PracticeHeading,
    };

    fn observation(
        candidate: &str,
        text: &str,
        recorded_in_audit: bool,
        present_in_practice_file: bool,
    ) -> LearningObservation {
        LearningObservation::new(
            Learning::new(
                LearningCandidateId::parse(candidate).expect("candidate"),
                LearningScope::Project,
                PracticeHeading::corrections(),
                text,
                LearningSource::Orchestrator,
            ),
            recorded_in_audit,
            present_in_practice_file,
        )
    }

    #[test]
    fn nothing_selected_captures_nothing() {
        assert!(LearningObservations::empty().captured().is_empty());
    }

    #[test]
    fn a_fresh_selection_is_captured_as_fresh() {
        let captured =
            LearningObservations::new(vec![observation("c1", "学び", false, false)]).captured();
        assert_eq!(captured.len(), 1);
        assert_eq!(
            captured
                .at(0)
                .map(super::super::CapturedLearning::disposition),
            Some(LearningDisposition::Fresh)
        );
        assert_eq!(captured.audit_rows(), 1);
    }

    /// 両側に在る学びは事実にならない（本家の no-op 分岐）。
    #[test]
    fn a_selection_present_on_both_sides_is_dropped() {
        let captured =
            LearningObservations::new(vec![observation("c1", "学び", true, true)]).captured();
        assert!(captured.is_empty());
        assert_eq!(captured.audit_rows(), 0);
    }

    /// 同じ回に同じ本文が 2 度現れたら最初の 1 件だけを残す（候補番号が違っても同じ本文）。
    #[test]
    fn the_same_text_twice_in_one_batch_is_captured_once() {
        let captured = LearningObservations::new(vec![
            observation("c1", "同じ本文", false, false),
            observation("c2", "同じ本文", false, false),
            observation("c3", "別の本文", false, false),
        ])
        .captured();
        assert_eq!(captured.len(), 2);
        assert_eq!(
            captured.at(0).map(|c| c.learning().text()),
            Some("同じ本文")
        );
        assert_eq!(
            captured.at(0).map(|c| c.learning().candidate_id().as_str()),
            Some("c1")
        );
        assert_eq!(
            captured.at(1).map(|c| c.learning().text()),
            Some("別の本文")
        );
        assert_eq!(captured.audit_rows(), 2);
    }

    /// 片側だけ欠けた状態からの復旧 — 欠けた側だけを補う。
    #[test]
    fn a_missing_side_is_repaired_without_duplicating_the_other() {
        let captured = LearningObservations::new(vec![
            observation("c1", "監査だけ在る", true, false),
            observation("c2", "実践行だけ在る", false, true),
        ])
        .captured();
        assert_eq!(captured.len(), 2);
        assert_eq!(
            captured
                .at(0)
                .map(super::super::CapturedLearning::disposition),
            Some(LearningDisposition::PracticeLineOnly)
        );
        assert_eq!(
            captured
                .at(1)
                .map(super::super::CapturedLearning::disposition),
            Some(LearningDisposition::AuditRowOnly)
        );
        assert_eq!(captured.audit_rows(), 1);
    }
}
