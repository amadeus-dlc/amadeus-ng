//! `MemoryJournalSurvey` — compile 時に読んだステージ日誌の観測列。

use core_infrastructure::collections::FirstClassCollection;

use super::stage_memory_journal::StageMemoryJournal;
use crate::workflow_definition::StageSlug;

/// runtime-graph の compile が読んだ日誌観測の列 (観測順)。
///
/// **素通しの列**である — 並べ替えも重複除去もしない。日誌が在ったステージだけが要素に
/// なるので、載っていないことは「日誌が無い」を意味する (件数 0 とは区別する)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryJournalSurvey {
    items: Vec<StageMemoryJournal>,
}

impl Default for MemoryJournalSurvey {
    fn default() -> Self {
        Self::of_items(Default::default())
    }
}

impl MemoryJournalSurvey {
    // 検査済みの列とその部分列は、この構築口で全状態を初期化する。
    const fn of_items(items: Vec<StageMemoryJournal>) -> Self {
        Self { items }
    }

    /// 日誌を 1 つも読めなかった観測。
    #[must_use]
    pub const fn empty() -> MemoryJournalSurvey {
        MemoryJournalSurvey::of_items(Vec::new())
    }

    /// 観測順のまま列にする (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(items: Vec<StageMemoryJournal>) -> MemoryJournalSurvey {
        MemoryJournalSurvey::of_items(items)
    }

    /// 観測の件数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }

    /// 観測が 1 件も無いか。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 観測順の添字参照。範囲外は `None` (panic しない)。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&StageMemoryJournal> {
        self.items.get(index)
    }

    /// 観測順に左から畳み込む。空なら初期値を返す。
    pub fn fold_left<'a, A>(
        &'a self,
        initial: A,
        fold: impl FnMut(A, &'a StageMemoryJournal) -> A,
    ) -> A {
        self.items.iter().fold(initial, fold)
    }

    /// 条件に一致する観測を観測順のまま残す。結果は空になり得る。
    #[must_use]
    pub fn filter(&self, mut predicate: impl FnMut(&StageMemoryJournal) -> bool) -> Self {
        MemoryJournalSurvey::new(
            self.items
                .iter()
                .filter(|journal| predicate(journal))
                .cloned()
                .collect(),
        )
    }

    /// その位置の観測 (最初の一致。日誌が無ければ `None`)。
    #[must_use]
    pub fn find(&self, stage: &StageSlug) -> Option<&StageMemoryJournal> {
        self.items.iter().find(|journal| journal.is_for(stage))
    }
}

impl FirstClassCollection for MemoryJournalSurvey {
    type Item<'a> = &'a StageMemoryJournal;
    type Filtered = Self;
    fn len(&self) -> usize {
        Self::len(self)
    }
    fn at(&self, index: usize) -> Option<&StageMemoryJournal> {
        Self::at(self, index)
    }
    fn fold_left<'a, A>(
        &'a self,
        initial: A,
        fold: impl FnMut(A, &'a StageMemoryJournal) -> A,
    ) -> A {
        Self::fold_left(self, initial, fold)
    }
    fn filter(&self, predicate: impl FnMut(&StageMemoryJournal) -> bool) -> Self {
        Self::filter(self, predicate)
    }
}

#[cfg(test)]
mod tests {
    use super::{MemoryJournalSurvey, StageMemoryJournal, StageSlug};
    use crate::orchestration::MemoryJournal;

    fn slug(name: &str) -> StageSlug {
        StageSlug::parse(name).expect("slug")
    }

    fn survey() -> MemoryJournalSurvey {
        MemoryJournalSurvey::new(vec![
            StageMemoryJournal::new(slug("state-init"), MemoryJournal::new(0, 0, 0, 0)),
            StageMemoryJournal::new(slug("reverse-engineering"), MemoryJournal::new(2, 0, 0, 1)),
        ])
    }

    #[test]
    fn the_empty_survey_carries_no_observation() {
        let empty = MemoryJournalSurvey::empty();
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
        assert_eq!(empty.at(0), None);
        assert_eq!(empty.fold_left(0, |count, _| count + 1), 0);
        assert_eq!(empty.find(&slug("state-init")), None);
    }

    #[test]
    fn the_survey_keeps_the_observation_order() {
        let survey = survey();
        assert_eq!(survey.len(), 2);
        assert_eq!(
            survey.at(0).map(StageMemoryJournal::stage),
            Some(&slug("state-init"))
        );
        assert_eq!(
            survey.at(1).map(StageMemoryJournal::stage),
            Some(&slug("reverse-engineering"))
        );
        assert_eq!(survey.at(2), None);
    }

    #[test]
    fn the_survey_finds_the_observation_of_one_stage() {
        let survey = survey();
        assert!(
            survey
                .find(&slug("state-init"))
                .is_some_and(StageMemoryJournal::is_empty)
        );
        assert!(survey.find(&slug("requirements-analysis")).is_none());
    }

    #[test]
    fn filtering_keeps_the_order_and_may_become_empty() {
        let survey = survey();
        let empties = survey.filter(|journal| journal.is_empty());
        assert_eq!(empties.len(), 1);
        assert_eq!(
            empties.at(0).map(StageMemoryJournal::stage),
            Some(&slug("state-init"))
        );
        assert!(survey.filter(|_| false).is_empty());
    }
}
