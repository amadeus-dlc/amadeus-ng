//! `MemoryEntries` — ステージ日誌が数えた記録の列。

use core_infrastructure::collections::FirstClassCollection;

use super::memory_entry::MemoryEntry;
use super::memory_entry_heading::MemoryEntryHeading;
use super::memory_journal::MemoryJournal;

/// 日誌 `memory.md` の記録を出現順に並べた列。
///
/// **数える単位はこの型が所有する** — 件数だけを返す [`MemoryJournal`] はこの列の畳み込み
/// であり、`MemoryEntries::parse(raw).len() == MemoryJournal::parse(raw).total()` が構造から
/// 成り立つ (固定本家 2.7.1 `a277af21` は `parseMemoryEntries` と `parseMemoryHeadings` に
/// 同じ読み飛ばし規則を二重に書いてこの不変条件を注記で守っている)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryEntries {
    items: Vec<MemoryEntry>,
}

impl Default for MemoryEntries {
    fn default() -> Self {
        Self::of_items(Default::default())
    }
}

impl MemoryEntries {
    // 検査済みの列とその部分列は、この構築口で全状態を初期化する。
    const fn of_items(items: Vec<MemoryEntry>) -> Self {
        Self { items }
    }

    /// 記録が 1 件も無い列。
    #[must_use]
    pub const fn empty() -> MemoryEntries {
        MemoryEntries::of_items(Vec::new())
    }

    /// 出現順のまま列にする (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(items: Vec<MemoryEntry>) -> MemoryEntries {
        MemoryEntries::of_items(items)
    }

    /// 日誌の本文から記録を読む (**この型の読取構築口**)。
    ///
    /// 数えるのは 4 つの錨の下にある行のうち、空行・引用のみの行・HTML コメントのみの行・
    /// フェンス内の行・見出し行そのものを除いたものである。錨でない `## ` 見出しは直前の節を
    /// 打ち切る。錨を欠く日誌はその見出しを 0 件として読む — 失敗にしない。
    ///
    /// 固定本家 2.7.1 `a277af21` の `aidlc-lib.ts:21492-21556` に対応する。
    #[must_use]
    pub fn parse(raw: &str) -> MemoryEntries {
        let mut items: Vec<MemoryEntry> = Vec::new();
        let mut current: Option<MemoryEntryHeading> = None;
        let mut fenced = false;
        for line in raw
            .trim_start_matches('\u{feff}')
            .replace("\r\n", "\n")
            .lines()
        {
            if line.starts_with(FENCE) {
                fenced = !fenced;
                continue;
            }
            if fenced {
                continue;
            }
            if let Some(heading) = MemoryEntryHeading::of_anchor(line) {
                current = Some(heading);
                continue;
            }
            if line.starts_with(HEADING) {
                current = None;
                continue;
            }
            let Some(heading) = current else { continue };
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('>') {
                continue;
            }
            if trimmed.starts_with(COMMENT_OPEN) && trimmed.ends_with(COMMENT_CLOSE) {
                continue;
            }
            items.push(MemoryEntry::parse(heading, trimmed));
        }
        MemoryEntries::new(items)
    }

    /// 記録の件数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }

    /// 記録が 1 件も無いか。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 出現順の添字参照。範囲外は `None` (panic しない)。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&MemoryEntry> {
        self.items.get(index)
    }

    /// 出現順に左から畳み込む。空なら初期値を返す。
    pub fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, &'a MemoryEntry) -> A) -> A {
        self.items.iter().fold(initial, fold)
    }

    /// 条件に一致する記録を出現順のまま残す。結果は空になり得る。
    #[must_use]
    pub fn filter(&self, mut predicate: impl FnMut(&MemoryEntry) -> bool) -> Self {
        MemoryEntries::new(
            self.items
                .iter()
                .filter(|entry| predicate(entry))
                .cloned()
                .collect(),
        )
    }

    /// 4 見出しごとの件数へ畳む。
    #[must_use]
    pub fn journal(&self) -> MemoryJournal {
        self.fold_left(MemoryJournal::new(0, 0, 0, 0), |journal, entry| {
            journal.counting(entry.heading())
        })
    }
}

/// コードフェンスの開閉。
const FENCE: &str = "```";
/// 錨でない H2 (直前の節を打ち切る)。
const HEADING: &str = "## ";
/// HTML コメントのみの行。
const COMMENT_OPEN: &str = "<!--";
/// 同上。
const COMMENT_CLOSE: &str = "-->";

impl FirstClassCollection for MemoryEntries {
    type Item<'a> = &'a MemoryEntry;
    type Filtered = Self;
    fn len(&self) -> usize {
        Self::len(self)
    }
    fn at(&self, index: usize) -> Option<&MemoryEntry> {
        Self::at(self, index)
    }
    fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, &'a MemoryEntry) -> A) -> A {
        Self::fold_left(self, initial, fold)
    }
    fn filter(&self, predicate: impl FnMut(&MemoryEntry) -> bool) -> Self {
        Self::filter(self, predicate)
    }
}

#[cfg(test)]
mod tests {
    use super::{MemoryEntries, MemoryEntryHeading};
    use crate::orchestration::MemoryJournal;

    const SAMPLE: &str = "# Stage Memory\n\n## Interpretations\n\n- 2026-05-20T10:14:32Z — 解釈; 文脈\n> 引用\n<!-- コメント -->\n\n## Deviations\n\n- 逸脱 1\n- 逸脱 2\n\n## Notes\n\n- 数えない\n\n## Tradeoffs\n\n```\n- フェンス内\n```\n\n## Open questions\n\n- 2026-05-20T11:00:00Z — 確認; 次回まで\n";

    #[test]
    fn the_entries_keep_the_order_and_their_headings() {
        let entries = MemoryEntries::parse(SAMPLE);
        assert_eq!(entries.len(), 4);
        let first = entries.at(0).expect("1 件目");
        assert_eq!(first.heading(), MemoryEntryHeading::Interpretations);
        assert_eq!(first.timestamp(), "2026-05-20T10:14:32Z");
        assert_eq!(first.summary(), "解釈");
        assert_eq!(first.context(), "文脈");
        assert_eq!(entries.at(1).map(|entry| entry.summary()), Some("逸脱 1"));
        assert_eq!(entries.at(2).map(|entry| entry.summary()), Some("逸脱 2"));
        let parked = entries.at(3).expect("4 件目");
        assert_eq!(parked.heading(), MemoryEntryHeading::OpenQuestions);
        assert!(parked.is_parked());
    }

    /// 件数の不変条件 — 読み飛ばし規則が 1 か所にしか無いことの検査。
    #[test]
    fn the_entry_count_equals_the_journal_total() {
        for raw in [
            SAMPLE,
            "",
            "## Interpretations\n\n## Deviations\n\n## Tradeoffs\n\n## Open questions\n",
            "\u{feff}## Deviations\r\n\r\n- 記録\r\n",
            "## Tradeoffs\n- a\n- b\n## Open questions\n- c\n",
        ] {
            assert_eq!(
                MemoryEntries::parse(raw).len() as u64,
                MemoryJournal::parse(raw).total(),
                "不変条件が崩れた: {raw:?}"
            );
            assert_eq!(
                MemoryEntries::parse(raw).journal(),
                MemoryJournal::parse(raw)
            );
        }
    }

    #[test]
    fn a_journal_without_any_entry_reads_as_an_empty_list() {
        assert!(MemoryEntries::parse("# Stage Memory\n").is_empty());
        assert_eq!(MemoryEntries::empty().len(), 0);
    }
}
