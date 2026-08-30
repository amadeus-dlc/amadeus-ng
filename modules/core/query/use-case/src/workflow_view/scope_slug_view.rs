//! `ScopeSlugView` — scope 名の文法 (`/^[a-z][a-z0-9-]*$/`、[`super::StageSlugView`] と同型)。
//!
//! scope の**存在** (定義に載っているか) は [`super::DefinitionView::is_valid_scope`] が
//! 判定する。本型は綴りの文法だけを守る (parse-don't-validate — 不正な綴りはこの型に
//! 存在しない)。

use std::fmt;

/// パース済みの scope 名 (不正値はこの型に存在しない)。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScopeSlugView(String);

/// `ScopeSlugView::parse` が拒否する文法違反。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeSlugError {
    /// 入力が空文字列。
    Empty,
    /// 先頭は `[a-z]` 必須。
    InvalidLeading(char),
    /// 2 文字目以降は `[a-z0-9-]` のみ。
    InvalidChar(char),
}

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

impl fmt::Display for ScopeSlugError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScopeSlugError::Empty => f.write_str("empty"),
            ScopeSlugError::InvalidLeading(c) => write!(f, "leading character '{c}'"),
            ScopeSlugError::InvalidChar(c) => write!(f, "invalid character '{c}'"),
        }
    }
}

impl std::error::Error for ScopeSlugError {}

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
    fn the_rejection_carries_material_not_wording() {
        assert_eq!(ScopeSlugError::Empty.to_string(), "empty");
        assert_eq!(
            ScopeSlugError::InvalidLeading('C').to_string(),
            "leading character 'C'"
        );
        assert_eq!(
            ScopeSlugError::InvalidChar('_').to_string(),
            "invalid character '_'"
        );
        let boxed: Box<dyn std::error::Error> = Box::new(ScopeSlugError::Empty);
        assert_eq!(boxed.to_string(), "empty");
    }

    #[test]
    fn display_writes_the_bare_name() {
        assert_eq!(
            ScopeSlugView::parse("classic").unwrap().to_string(),
            "classic"
        );
    }
}
