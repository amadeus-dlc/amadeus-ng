//! `UnknownUnitKind` — [`UnitKind::parse`] が拒否した未知の種別語。
//!
//! 運ぶのは拒否された生値だけで、利用者向けの逐語文言は出す側が組む
//! (`coding-rules/error-handling.md`)。
//!
//! [`UnitKind::parse`]: super::UnitKind::parse

use std::fmt;

/// `UnitKind::parse` が拒否する未知の種別語。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownUnitKind(String);

impl UnknownUnitKind {
    /// 拒否された生値をそのまま包む。
    #[must_use]
    pub(super) fn new(value: &str) -> UnknownUnitKind {
        UnknownUnitKind(value.to_string())
    }
}

impl fmt::Display for UnknownUnitKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown unit kind \"{}\"", self.0)
    }
}

impl std::error::Error for UnknownUnitKind {}
