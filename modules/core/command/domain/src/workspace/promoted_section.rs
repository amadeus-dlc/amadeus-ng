//! `PromotedSection` — 昇格が team.md の 1 節へ書き写す見出しと本文の対。

/// 昇格で置き換える節 1 つ (見出し名と、そこへ書く本文)。
///
/// `heading` は `## ` を**除いた**名前である (`Way of Working`) — upstream の
/// `sectionsWritten.push(heading.slice(3))` と同じ綴りで、stdout の `sections_written` と
/// 監査行の `Sections Written` に出る値そのものである。書き込むときに `## ` を前置するのは
/// 投影側 ([`super::replace_section`] の引数を組む場所) である。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotedSection {
    heading: String,
    body: String,
}

impl PromotedSection {
    /// 見出し名と本文から組む (**この型の唯一の構築経路**)。
    #[must_use]
    pub fn new(heading: impl Into<String>, body: impl Into<String>) -> PromotedSection {
        PromotedSection {
            heading: heading.into(),
            body: body.into(),
        }
    }

    /// 見出し名 (`## ` を含まない)。
    #[must_use]
    pub fn heading(&self) -> &str {
        &self.heading
    }

    /// その節へ書く本文。
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_section_carries_its_bare_heading_and_body() {
        let section = PromotedSection::new("Way of Working", "trunk-based.\n");
        assert_eq!(section.heading(), "Way of Working");
        assert_eq!(section.body(), "trunk-based.\n");
    }
}
