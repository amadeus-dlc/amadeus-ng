//! `UnitRef` — per-unit 反復の unit 名指し (名前 + 種別の対)。
//!
//! 「名前だけあって種別が無い」という不正状態は対の型で表現不能になる。種別は units YAML の
//! 閉じた語彙 (`service | spec | ui | packaging | library`) なので enum で運ぶ。

use std::fmt;

/// unit 名の文法 (`/^[a-z][a-z0-9-]*$/` — 例 `u4-read-model-updater`)。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnitName(String);

/// `UnitName::parse` が拒否する文法違反。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitNameError {
    /// 入力が空文字列。
    Empty,
    /// 先頭は `[a-z]` 必須。
    InvalidLeading(char),
    /// 2 文字目以降は `[a-z0-9-]` のみ。
    InvalidChar(char),
}

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

impl fmt::Display for UnitNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnitNameError::Empty => f.write_str("empty"),
            UnitNameError::InvalidLeading(c) => write!(f, "leading character '{c}'"),
            UnitNameError::InvalidChar(c) => write!(f, "invalid character '{c}'"),
        }
    }
}

impl std::error::Error for UnitNameError {}

/// unit 種別 — units YAML の `kind` 閉集合 (`service | spec | ui | packaging | library`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitKind {
    /// `service`。
    Service,
    /// `spec`。
    Spec,
    /// `ui`。
    Ui,
    /// `packaging`。
    Packaging,
    /// `library`。
    Library,
}

/// `UnitKind::parse` が拒否する未知の種別語。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownUnitKind(String);

impl UnitKind {
    /// # Errors
    ///
    /// 閉集合 5 語以外を拒否する。
    pub fn parse(s: &str) -> Result<UnitKind, UnknownUnitKind> {
        match s {
            "service" => Ok(UnitKind::Service),
            "spec" => Ok(UnitKind::Spec),
            "ui" => Ok(UnitKind::Ui),
            "packaging" => Ok(UnitKind::Packaging),
            "library" => Ok(UnitKind::Library),
            other => Err(UnknownUnitKind(other.to_string())),
        }
    }

    /// 固定トークン綴り。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            UnitKind::Service => "service",
            UnitKind::Spec => "spec",
            UnitKind::Ui => "ui",
            UnitKind::Packaging => "packaging",
            UnitKind::Library => "library",
        }
    }
}

impl fmt::Display for UnknownUnitKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown unit kind \"{}\"", self.0)
    }
}

impl std::error::Error for UnknownUnitKind {}

/// unit の名指し — 名前と種別の対。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitRef {
    name: UnitName,
    kind: UnitKind,
}

impl UnitRef {
    /// 名前と種別を束ねる。
    #[must_use]
    pub const fn new(name: UnitName, kind: UnitKind) -> UnitRef {
        UnitRef { name, kind }
    }

    /// unit 名。
    #[must_use]
    pub const fn name(&self) -> &UnitName {
        &self.name
    }

    /// unit 種別。
    #[must_use]
    pub const fn kind(&self) -> UnitKind {
        self.kind
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

    #[test]
    fn the_name_rejection_carries_material_not_wording() {
        assert_eq!(UnitNameError::Empty.to_string(), "empty");
        assert_eq!(
            UnitNameError::InvalidLeading('U').to_string(),
            "leading character 'U'"
        );
        assert_eq!(
            UnitNameError::InvalidChar('_').to_string(),
            "invalid character '_'"
        );
        let boxed: Box<dyn std::error::Error> = Box::new(UnitNameError::Empty);
        assert_eq!(boxed.to_string(), "empty");
    }

    #[test]
    fn the_five_kinds_round_trip_and_unknown_is_rejected() {
        for kind in [
            UnitKind::Service,
            UnitKind::Spec,
            UnitKind::Ui,
            UnitKind::Packaging,
            UnitKind::Library,
        ] {
            assert_eq!(UnitKind::parse(kind.as_str()), Ok(kind));
        }
        let error = UnitKind::parse("weird").unwrap_err();
        assert_eq!(error.to_string(), "unknown unit kind \"weird\"");
        let boxed: Box<dyn std::error::Error> = Box::new(error);
        assert_eq!(boxed.to_string(), "unknown unit kind \"weird\"");
    }

    #[test]
    fn the_pair_carries_both_halves() {
        let unit = UnitRef::new(
            UnitName::parse("u3-event-store").unwrap(),
            UnitKind::Library,
        );
        assert_eq!(unit.name().as_str(), "u3-event-store");
        assert_eq!(unit.kind(), UnitKind::Library);
    }
}
