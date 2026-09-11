//! `CapturedLearnings` — 記録すると決まった学びの列。

use core_infrastructure::collections::FirstClassCollection;

use super::captured_learning::CapturedLearning;

/// 集約が「この回で書く」と決めた学びの列（選択順）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedLearnings {
    items: Vec<CapturedLearning>,
}

impl Default for CapturedLearnings {
    fn default() -> Self {
        Self::of_items(Default::default())
    }
}

impl CapturedLearnings {
    // 検査済みの列とその部分列は、この構築口で全状態を初期化する。
    const fn of_items(items: Vec<CapturedLearning>) -> Self {
        Self { items }
    }

    /// 書くものが 1 件も無い判定。
    #[must_use]
    pub const fn empty() -> CapturedLearnings {
        CapturedLearnings::of_items(Vec::new())
    }

    /// 選択順のまま列にする（**この型の唯一の構築経路**）。
    #[must_use]
    pub const fn new(items: Vec<CapturedLearning>) -> CapturedLearnings {
        CapturedLearnings::of_items(items)
    }

    /// 書く学びの件数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }

    /// 書くものが 1 件も無いか。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 選択順の添字参照。範囲外は `None`（panic しない）。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&CapturedLearning> {
        self.items.get(index)
    }

    /// 選択順に左から畳み込む。空なら初期値を返す。
    pub fn fold_left<'a, A>(
        &'a self,
        initial: A,
        fold: impl FnMut(A, &'a CapturedLearning) -> A,
    ) -> A {
        self.items.iter().fold(initial, fold)
    }

    /// 条件に一致する学びを選択順のまま残す。結果は空になり得る。
    #[must_use]
    pub fn filter(&self, mut predicate: impl FnMut(&CapturedLearning) -> bool) -> Self {
        CapturedLearnings::new(
            self.items
                .iter()
                .filter(|captured| predicate(captured))
                .cloned()
                .collect(),
        )
    }

    /// 監査行が立つ件数（persist が返す `rule_learned`）。
    #[must_use]
    pub fn audit_rows(&self) -> usize {
        self.fold_left(0, |count, captured| {
            if captured.disposition().writes_audit_row() {
                count + 1
            } else {
                count
            }
        })
    }
}

impl FirstClassCollection for CapturedLearnings {
    type Item<'a> = &'a CapturedLearning;
    type Filtered = Self;
    fn len(&self) -> usize {
        Self::len(self)
    }
    fn at(&self, index: usize) -> Option<&CapturedLearning> {
        Self::at(self, index)
    }
    fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, &'a CapturedLearning) -> A) -> A {
        Self::fold_left(self, initial, fold)
    }
    fn filter(&self, predicate: impl FnMut(&CapturedLearning) -> bool) -> Self {
        Self::filter(self, predicate)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::CapturedLearnings;
    use crate::orchestration::{
        CapturedLearning, Learning, LearningCandidateId, LearningDisposition, LearningScope,
        LearningSource, PracticeHeading,
    };

    fn captured(text: &str, disposition: LearningDisposition) -> CapturedLearning {
        CapturedLearning::new(
            Learning::new(
                LearningCandidateId::parse("c1").expect("candidate"),
                LearningScope::Project,
                PracticeHeading::corrections(),
                text,
                LearningSource::Orchestrator,
            ),
            disposition,
        )
    }

    #[test]
    fn an_empty_capture_writes_nothing() {
        let empty = CapturedLearnings::empty();
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        assert_eq!(empty.audit_rows(), 0);
        assert_eq!(empty.at(0), None);
    }

    #[test]
    fn only_the_rows_that_land_in_the_audit_are_counted() {
        let learnings = CapturedLearnings::new(vec![
            captured("a", LearningDisposition::Fresh),
            captured("b", LearningDisposition::PracticeLineOnly),
            captured("c", LearningDisposition::AuditRowOnly),
        ]);
        assert_eq!(learnings.len(), 3);
        assert_eq!(learnings.audit_rows(), 2);
    }

    #[test]
    fn filtering_keeps_the_selection_order_and_may_become_empty() {
        let learnings = CapturedLearnings::new(vec![
            captured("a", LearningDisposition::Fresh),
            captured("b", LearningDisposition::AuditRowOnly),
        ]);
        let fresh =
            learnings.filter(|captured| captured.disposition() == LearningDisposition::Fresh);
        assert_eq!(fresh.len(), 1);
        assert_eq!(fresh.at(0).map(|c| c.learning().text()), Some("a"));
        assert!(learnings.filter(|_| false).is_empty());
    }
}
