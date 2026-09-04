//! `HeadingNotFound` — Markdown 節操作 (`replace_section` / `append_under_heading`) の拒否。

/// 節操作の拒否 — 対象の `## Heading` が本文に存在しない。
///
/// 運ぶのは**見つからなかった見出しの綴り**だけである (`## ` を含む完全形)。upstream の
/// `replaceSection: heading not found: ## Mandated` のような文言を組むのは出す側であり、
/// この型は材料しか持たない (`coding-rules/error-handling.md`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadingNotFound(String);

impl HeadingNotFound {
    /// 見つからなかった見出しの綴り (`## ` を含む完全形) から構成する。
    #[must_use]
    pub fn new(heading: impl Into<String>) -> HeadingNotFound {
        HeadingNotFound(heading.into())
    }

    /// 見つからなかった見出しの綴り (`## ` を含む完全形)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl core::fmt::Display for HeadingNotFound {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "heading not found: {}", self.0)
    }
}

impl std::error::Error for HeadingNotFound {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_refusal_carries_the_heading_spelling() {
        let error = HeadingNotFound::new("## Mandated");
        assert_eq!(error.as_str(), "## Mandated");
        assert_eq!(error.to_string(), "heading not found: ## Mandated");
    }
}
