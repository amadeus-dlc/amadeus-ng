//! `StageMemoryJournal` — どのステージの日誌がどれだけ書かれているか。

use super::memory_journal::MemoryJournal;
use crate::workflow_definition::StageSlug;

/// 1 ステージ分の日誌観測 (位置と件数の対)。
///
/// 日誌が**在った**ステージだけがこの値になる。日誌が無いステージは
/// [`MemoryJournalSurvey`] に載らない — 固定本家 2.7.1 の `readMemory` が
/// ファイル不在を `null` として `MEMORY_EMPTY` の対象から外すのと同じ区別である。
///
/// [`MemoryJournalSurvey`]: super::MemoryJournalSurvey
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageMemoryJournal {
    stage: StageSlug,
    journal: MemoryJournal,
}

impl StageMemoryJournal {
    /// ステージと日誌件数を束ねる完全コンストラクタ。
    #[must_use]
    pub const fn new(stage: StageSlug, journal: MemoryJournal) -> Self {
        Self { stage, journal }
    }

    /// 観測したステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }

    /// 観測した日誌の件数。
    #[must_use]
    pub const fn journal(&self) -> MemoryJournal {
        self.journal
    }

    /// この位置の日誌が空か。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.journal.is_empty()
    }

    /// 与えられた位置の観測か。
    #[must_use]
    pub fn is_for(&self, stage: &StageSlug) -> bool {
        &self.stage == stage
    }
}

#[cfg(test)]
mod tests {
    use super::{MemoryJournal, StageMemoryJournal, StageSlug};

    fn slug(name: &str) -> StageSlug {
        StageSlug::parse(name).expect("slug")
    }

    #[test]
    fn the_observation_keeps_its_stage_and_counts() {
        let observation =
            StageMemoryJournal::new(slug("state-init"), MemoryJournal::new(0, 1, 0, 0));
        assert_eq!(observation.stage(), &slug("state-init"));
        assert_eq!(observation.journal().total(), 1);
        assert!(!observation.is_empty());
        assert!(observation.is_for(&slug("state-init")));
        assert!(!observation.is_for(&slug("requirements-analysis")));
    }

    #[test]
    fn an_observation_without_entries_reports_empty() {
        let observation =
            StageMemoryJournal::new(slug("state-init"), MemoryJournal::new(0, 0, 0, 0));
        assert!(observation.is_empty());
    }
}
