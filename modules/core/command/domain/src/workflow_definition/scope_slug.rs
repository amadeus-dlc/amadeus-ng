//! `ScopeSlug` — scope 名の文法 (`/^[a-z][a-z0-9-]*$/`、`StageSlug` と同型)。
//!
//! scope の**存在** (定義に載っているか) は `WorkflowDefinition::is_valid_scope` が判定する。
//! 本型は綴りの文法だけを守る (parse-don't-validate — 不正な綴りはこの型に存在しない)。

use std::fmt;

use super::scope_slug_error::ScopeSlugError;

/// パース済みの scope 名 (Always Valid)。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScopeSlug(String);

impl ScopeSlug {
    /// # Errors
    ///
    /// 空・先頭非 `[a-z]`・`[a-z0-9-]` 以外の文字を拒否する。
    pub fn parse(s: &str) -> Result<ScopeSlug, ScopeSlugError> {
        let mut chars = s.chars();
        match chars.next() {
            None => return Err(ScopeSlugError::Empty),
            Some(c) if !c.is_ascii_lowercase() => return Err(ScopeSlugError::InvalidLeading(c)),
            Some(_) => {}
        }
        for c in chars {
            if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                return Err(ScopeSlugError::InvalidChar(c));
            }
        }
        Ok(ScopeSlug(s.to_string()))
    }

    /// 生の scope 名 (正規化なし)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ScopeSlug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_slugs_parse_and_round_trip() {
        assert_eq!(ScopeSlug::parse("classic").unwrap().as_str(), "classic");
        assert_eq!(
            ScopeSlug::parse("security-patch").unwrap().to_string(),
            "security-patch"
        );
    }

    #[test]
    fn invalid_slugs_are_rejected() {
        assert_eq!(ScopeSlug::parse(""), Err(ScopeSlugError::Empty));
        assert_eq!(
            ScopeSlug::parse("1abc"),
            Err(ScopeSlugError::InvalidLeading('1'))
        );
        assert_eq!(
            ScopeSlug::parse("a_b"),
            Err(ScopeSlugError::InvalidChar('_'))
        );
        assert_eq!(
            ScopeSlugError::InvalidChar('_').to_string(),
            "invalid character '_'"
        );
        assert_eq!(ScopeSlugError::Empty.to_string(), "empty");
        assert_eq!(
            ScopeSlugError::InvalidLeading('1').to_string(),
            "leading character '1'"
        );
    }
}
