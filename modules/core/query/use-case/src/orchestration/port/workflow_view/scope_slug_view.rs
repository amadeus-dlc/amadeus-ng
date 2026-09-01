//! `ScopeSlugView` — scope 名の文法 (`/^[a-z][a-z0-9-]*$/`、[`super::StageSlugView`] と同型)。
//!
//! scope の**存在** (定義に載っているか) は [`super::DefinitionView::is_valid_scope`] が
//! 判定する。本型は綴りの文法だけを守る (parse-don't-validate — 不正な綴りはこの型に
//! 存在しない)。

use std::fmt;

use super::scope_slug_error::ScopeSlugError;

/// パース済みの scope 名 (不正値はこの型に存在しない)。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScopeSlugView(String);

impl ScopeSlugView {
    /// # Errors
    ///
    /// 空・先頭非 `[a-z]`・`[a-z0-9-]` 以外の文字を拒否する。
    pub fn parse(s: &str) -> Result<ScopeSlugView, ScopeSlugError> {
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
        Ok(ScopeSlugView(s.to_string()))
    }

    /// 生の scope 名 (正規化なし)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ScopeSlugView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_shipping_vocabulary() {
        for s in ["classic", "bugfix", "security-patch", "mvp"] {
            assert_eq!(ScopeSlugView::parse(s).unwrap().as_str(), s);
        }
    }

    #[test]
    fn rejects_shapes_that_could_not_be_a_scope_file_stem() {
        assert_eq!(ScopeSlugView::parse(""), Err(ScopeSlugError::Empty));
        assert_eq!(
            ScopeSlugView::parse("Classic"),
            Err(ScopeSlugError::InvalidLeading('C'))
        );
        assert_eq!(
            ScopeSlugView::parse("1scope"),
            Err(ScopeSlugError::InvalidLeading('1'))
        );
        assert_eq!(
            ScopeSlugView::parse("security_patch"),
            Err(ScopeSlugError::InvalidChar('_'))
        );
    }

    #[test]
    fn display_writes_the_bare_name() {
        assert_eq!(
            ScopeSlugView::parse("classic").unwrap().to_string(),
            "classic"
        );
    }
}
