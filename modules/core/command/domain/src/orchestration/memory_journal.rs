//! `MemoryJournal` — 1 ステージの観測日誌に記録された件数。

/// ステージ日誌 `memory.md` の 4 見出しごとの記録件数。
///
/// 見出しは `## Interpretations` / `## Deviations` / `## Tradeoffs` /
/// `## Open questions` の 4 本で、固定本家 2.7.1 `a277af21` の
/// `aidlc-lib.ts:21414-21477` (`parseMemoryHeadings`) が数える単位に一致する。
/// **日誌が在るのに 1 件も無い**ことは、この型が `is_empty` で答える (日誌そのものが
/// 無い場合は survey に載らない — 観測できないものを 0 件と偽らない)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryJournal {
    interpretations: u64,
    deviations: u64,
    tradeoffs: u64,
    open_questions: u64,
}

impl MemoryJournal {
    /// 4 見出しの件数を束ねる完全コンストラクタ。
    #[must_use]
    pub const fn new(
        interpretations: u64,
        deviations: u64,
        tradeoffs: u64,
        open_questions: u64,
    ) -> Self {
        Self {
            interpretations,
            deviations,
            tradeoffs,
            open_questions,
        }
    }

    /// `## Interpretations` の件数。
    #[must_use]
    pub const fn interpretations(&self) -> u64 {
        self.interpretations
    }

    /// `## Deviations` の件数。
    #[must_use]
    pub const fn deviations(&self) -> u64 {
        self.deviations
    }

    /// `## Tradeoffs` の件数。
    #[must_use]
    pub const fn tradeoffs(&self) -> u64 {
        self.tradeoffs
    }

    /// `## Open questions` の件数。
    #[must_use]
    pub const fn open_questions(&self) -> u64 {
        self.open_questions
    }

    /// 4 見出しの合計件数。
    #[must_use]
    pub const fn total(&self) -> u64 {
        self.interpretations + self.deviations + self.tradeoffs + self.open_questions
    }

    /// 日誌に 1 件も記録が無いか (`MEMORY_EMPTY` の判定材料)。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.total() == 0
    }

    /// 見出し 1 件ぶんを数え上げた新しい件数 (値オブジェクトなので自身は変えない)。
    #[must_use]
    pub const fn counting(&self, heading: super::MemoryEntryHeading) -> MemoryJournal {
        match heading {
            super::MemoryEntryHeading::Interpretations => MemoryJournal::new(
                self.interpretations.saturating_add(1),
                self.deviations,
                self.tradeoffs,
                self.open_questions,
            ),
            super::MemoryEntryHeading::Deviations => MemoryJournal::new(
                self.interpretations,
                self.deviations.saturating_add(1),
                self.tradeoffs,
                self.open_questions,
            ),
            super::MemoryEntryHeading::Tradeoffs => MemoryJournal::new(
                self.interpretations,
                self.deviations,
                self.tradeoffs.saturating_add(1),
                self.open_questions,
            ),
            super::MemoryEntryHeading::OpenQuestions => MemoryJournal::new(
                self.interpretations,
                self.deviations,
                self.tradeoffs,
                self.open_questions.saturating_add(1),
            ),
        }
    }

    /// 日誌の本文から 4 見出しの記録件数を数える (**この型の読取構築口**)。
    ///
    /// 読み飛ばし規則は [`MemoryEntries::parse`] が唯一の所有者である — 件数はその列の
    /// 畳み込みなので、`MemoryEntries::parse(raw).len() == MemoryJournal::parse(raw).total()`
    /// が構造から成り立つ (本家は同じ規則を 2 か所へ書き、注記でこの不変条件を守っている)。
    ///
    /// 固定本家 2.7.1 `a277af21` の `aidlc-lib.ts:21414-21477` (`parseMemoryHeadings`) に対応する。
    ///
    /// [`MemoryEntries::parse`]: super::MemoryEntries::parse
    #[must_use]
    pub fn parse(raw: &str) -> MemoryJournal {
        super::MemoryEntries::parse(raw).journal()
    }
}

#[cfg(test)]
mod tests {
    use super::MemoryJournal;

    #[test]
    fn the_journal_sums_the_four_headings() {
        let journal = MemoryJournal::new(1, 2, 3, 4);
        assert_eq!(journal.interpretations(), 1);
        assert_eq!(journal.deviations(), 2);
        assert_eq!(journal.tradeoffs(), 3);
        assert_eq!(journal.open_questions(), 4);
        assert_eq!(journal.total(), 10);
        assert!(!journal.is_empty());
    }

    #[test]
    fn a_journal_without_any_entry_is_empty() {
        assert!(MemoryJournal::new(0, 0, 0, 0).is_empty());
        assert!(!MemoryJournal::new(0, 0, 0, 1).is_empty());
    }

    #[test]
    fn the_four_anchors_are_counted_and_everything_else_is_skipped() {
        let raw = "# Stage Memory\n\n## Interpretations\n\n- 記録 1\n> 引用\n<!-- コメント -->\n\n## Deviations\n\n- 記録 2\n- 記録 3\n\n## Notes\n\n- 数えない\n\n## Tradeoffs\n\n```\n- フェンス内\n```\n\n## Open questions\n\n- 記録 4\n";
        let journal = MemoryJournal::parse(raw);
        assert_eq!(journal.interpretations(), 1);
        assert_eq!(journal.deviations(), 2);
        assert_eq!(journal.tradeoffs(), 0);
        assert_eq!(journal.open_questions(), 1);
        assert_eq!(journal.total(), 4);
    }

    #[test]
    fn a_journal_with_only_the_anchors_reads_as_empty() {
        assert!(
            MemoryJournal::parse(
                "## Interpretations\n\n## Deviations\n\n## Tradeoffs\n\n## Open questions\n"
            )
            .is_empty()
        );
    }

    #[test]
    fn a_journal_that_lacks_an_anchor_reads_that_heading_as_zero() {
        let journal = MemoryJournal::parse("## Interpretations\n\n- 記録\n");
        assert_eq!(journal.interpretations(), 1);
        assert_eq!(journal.deviations(), 0);
        assert_eq!(journal.total(), 1);
    }

    #[test]
    fn a_bom_and_crlf_line_endings_do_not_change_the_count() {
        let journal = MemoryJournal::parse("\u{feff}## Deviations\r\n\r\n- 記録\r\n");
        assert_eq!(journal.deviations(), 1);
    }
}
