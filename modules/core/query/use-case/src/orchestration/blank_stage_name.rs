//! `BlankStageName` — [`StageName::parse`] が空・空白のみを拒否した形。
//!
//! 運ぶ材料は無い — 空白のみの名前が名指しとして成立しないことは型名そのものが述べており、
//! 利用者向けの逐語文言は出す側が組む (`coding-rules/error-handling.md`)。
//!
//! [`StageName::parse`]: super::StageName::parse

use std::fmt;

/// `StageName::parse` が拒否する形。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlankStageName;

impl fmt::Display for BlankStageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("blank stage name")
    }
}

impl std::error::Error for BlankStageName {}
