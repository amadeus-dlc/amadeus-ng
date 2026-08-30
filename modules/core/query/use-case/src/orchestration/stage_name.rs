//! `StageName` — ステージの表示名 (例 "NFR Requirements")。
//!
//! slug ([`crate::workflow_view::StageSlugView`]) とは別語彙 — 表示名は人間向けの文言で、
//! グラフの同一性には使わない。空白のみの名前は名指しとして成立しないので拒否する。

use std::fmt;

/// パース済みのステージ表示名 (空・空白のみは存在しない)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageName(String);

/// `StageName::parse` が拒否する形。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlankStageName;

impl StageName {
    /// # Errors
    ///
    /// 空・空白のみを拒否する。
    pub fn parse(s: &str) -> Result<StageName, BlankStageName> {
        if s.trim().is_empty() {
            return Err(BlankStageName);
        }
        Ok(StageName(s.to_string()))
    }

    /// 生の表示名。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for BlankStageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("blank stage name")
    }
}

impl std::error::Error for BlankStageName {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_display_name_keeps_its_spelling() {
        let name = StageName::parse("NFR Requirements").unwrap();
        assert_eq!(name.as_str(), "NFR Requirements");
        assert_eq!(name.to_string(), "NFR Requirements");
    }

    #[test]
    fn blank_names_are_not_a_naming() {
        assert_eq!(StageName::parse(""), Err(BlankStageName));
        assert_eq!(StageName::parse("   \t"), Err(BlankStageName));
        assert_eq!(BlankStageName.to_string(), "blank stage name");
        let boxed: Box<dyn std::error::Error> = Box::new(BlankStageName);
        assert_eq!(boxed.to_string(), "blank stage name");
    }
}
