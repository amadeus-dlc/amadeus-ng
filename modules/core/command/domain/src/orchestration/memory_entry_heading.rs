//! `MemoryEntryHeading` — ステージ日誌 `memory.md` が数える 4 見出し。

/// 日誌の記録が属する見出し。
///
/// 綴りは固定本家 2.7.1 `a277af21` の `aidlc-lib.ts:21492-21556` (`parseMemoryEntries`) が
/// 出力する表示名そのもの (`Interpretations` / `Deviations` / `Tradeoffs` /
/// `Open questions`) であり、learnings の surface が出す JSON の `source_heading` に載る
/// 逐語である (Published Language — `coding-rules/upstream-contracts.md`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryEntryHeading {
    /// ステージ本文が曖昧だった箇所の解釈。
    Interpretations,
    /// ステージ本文から意図的に外れた箇所。
    Deviations,
    /// 検討した代替案と選んだ理由。
    Tradeoffs,
    /// 次回までに確かめること。**学びの候補には昇格しない**。
    OpenQuestions,
}

impl MemoryEntryHeading {
    /// 錨の行 (`## Interpretations` 等) が指す見出し。錨でなければ `None`。
    #[must_use]
    pub fn of_anchor(line: &str) -> Option<MemoryEntryHeading> {
        match line {
            "## Interpretations" => Some(MemoryEntryHeading::Interpretations),
            "## Deviations" => Some(MemoryEntryHeading::Deviations),
            "## Tradeoffs" => Some(MemoryEntryHeading::Tradeoffs),
            "## Open questions" => Some(MemoryEntryHeading::OpenQuestions),
            _ => None,
        }
    }

    /// 表示名の逐語 (surface の `source_heading`)。
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            MemoryEntryHeading::Interpretations => "Interpretations",
            MemoryEntryHeading::Deviations => "Deviations",
            MemoryEntryHeading::Tradeoffs => "Tradeoffs",
            MemoryEntryHeading::OpenQuestions => "Open questions",
        }
    }

    /// 学びの候補へ昇格しない見出しか (`Open questions` だけが真)。
    #[must_use]
    pub const fn is_parked(&self) -> bool {
        matches!(self, MemoryEntryHeading::OpenQuestions)
    }
}

#[cfg(test)]
mod tests {
    use super::MemoryEntryHeading;

    #[test]
    fn only_the_four_anchors_name_a_heading() {
        assert_eq!(
            MemoryEntryHeading::of_anchor("## Interpretations"),
            Some(MemoryEntryHeading::Interpretations)
        );
        assert_eq!(
            MemoryEntryHeading::of_anchor("## Open questions"),
            Some(MemoryEntryHeading::OpenQuestions)
        );
        assert_eq!(MemoryEntryHeading::of_anchor("## Notes"), None);
        assert_eq!(MemoryEntryHeading::of_anchor("## interpretations"), None);
    }

    #[test]
    fn the_display_spelling_is_the_upstream_verbatim() {
        assert_eq!(
            MemoryEntryHeading::Interpretations.as_str(),
            "Interpretations"
        );
        assert_eq!(MemoryEntryHeading::Deviations.as_str(), "Deviations");
        assert_eq!(MemoryEntryHeading::Tradeoffs.as_str(), "Tradeoffs");
        assert_eq!(MemoryEntryHeading::OpenQuestions.as_str(), "Open questions");
    }

    #[test]
    fn only_open_questions_are_parked() {
        assert!(MemoryEntryHeading::OpenQuestions.is_parked());
        assert!(!MemoryEntryHeading::Tradeoffs.is_parked());
    }
}
