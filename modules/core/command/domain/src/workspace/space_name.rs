//! `SpaceName` — `/^[a-z][a-z0-9-]*$/`。「生のまま `join()` に到達してはならないパスセグメント」
//! (upstream 03 §3.1, `aidlc-lib.ts:1341`)。

use std::fmt;

use super::space_name_error::SpaceNameError;

/// パース済みの space 名 (Always Valid — 不正値はこの型に存在しない)。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpaceName(String);

/// default space (ディスクに何もなくても常に有効 — upstream 特例)。
impl Default for SpaceName {
    fn default() -> SpaceName {
        SpaceName("default".to_string())
    }
}

impl SpaceName {
    /// # Errors
    ///
    /// 空・先頭非 `[a-z]`・`[a-z0-9-]` 以外の文字を拒否する。
    pub fn parse(s: &str) -> Result<SpaceName, SpaceNameError> {
        let mut chars = s.chars();
        match chars.next() {
            None => return Err(SpaceNameError::Empty),
            Some(c) if !c.is_ascii_lowercase() => return Err(SpaceNameError::InvalidLeading(c)),
            Some(_) => {}
        }
        for c in chars {
            if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                return Err(SpaceNameError::InvalidChar(c));
            }
        }
        Ok(SpaceName(s.to_string()))
    }

    /// 検証済みのパスセグメント — `join()` に渡してよい唯一の形。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SpaceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_kebab_case_names_and_the_default_impl() {
        assert!(SpaceName::parse("default").is_ok());
        assert!(SpaceName::parse("team-a2").is_ok());
        assert!(SpaceName::parse("x").is_ok());
    }

    #[test]
    fn rejects_path_hazards_before_they_can_reach_join() {
        assert!(SpaceName::parse("").is_err());
        assert!(SpaceName::parse("Team").is_err());
        assert!(SpaceName::parse("1team").is_err());
        assert!(SpaceName::parse("-team").is_err());
        assert!(SpaceName::parse("a/b").is_err());
        assert!(SpaceName::parse("a b").is_err());
        assert!(SpaceName::parse("..").is_err());
    }
}
