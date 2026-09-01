//! `UnitName` — per-unit 反復が名指しする unit の名前。
//!
//! 文法 (`/^[a-z][a-z0-9-]*$/`) は parse で一度だけ検査し、以後は型が保証する
//! (`coding-rules/factory-naming.md` — `parse` が唯一の入口)。

use std::fmt;

use super::unit_name_error::UnitNameError;

/// unit 名の文法 (`/^[a-z][a-z0-9-]*$/` — 例 `u4-read-model-updater`)。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnitName(String);

impl UnitName {
    /// # Errors
    ///
    /// 空・先頭非 `[a-z]`・`[a-z0-9-]` 以外の文字を拒否する。
    pub fn parse(s: &str) -> Result<UnitName, UnitNameError> {
        let mut chars = s.chars();
        match chars.next() {
            None => return Err(UnitNameError::Empty),
            Some(c) if !c.is_ascii_lowercase() => return Err(UnitNameError::InvalidLeading(c)),
            Some(_) => {}
        }
        for c in chars {
            if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                return Err(UnitNameError::InvalidChar(c));
            }
        }
        Ok(UnitName(s.to_string()))
    }

    /// 生の unit 名。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UnitName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_follows_the_slug_grammar() {
        assert_eq!(
            UnitName::parse("u6-next-continue-use-case")
                .unwrap()
                .as_str(),
            "u6-next-continue-use-case"
        );
        assert_eq!(UnitName::parse(""), Err(UnitNameError::Empty));
        assert_eq!(
            UnitName::parse("U6"),
            Err(UnitNameError::InvalidLeading('U'))
        );
        assert_eq!(
            UnitName::parse("u6_next"),
            Err(UnitNameError::InvalidChar('_'))
        );
        assert_eq!(UnitName::parse("u6").unwrap().to_string(), "u6");
    }
}
