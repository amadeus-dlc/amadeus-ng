//! `UnitRef` — per-unit 反復の unit 名指し (名前 + 種別の対)。
//!
//! 「名前だけあって種別が無い」という不正状態は対の型で表現不能になる。種別は units YAML の
//! 閉じた語彙 ([`UnitKind`]) なので enum で運ぶ。

use super::unit_kind::UnitKind;
use super::unit_name::UnitName;

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
    fn the_pair_carries_both_halves() {
        let unit = UnitRef::new(
            UnitName::parse("u3-event-store").unwrap(),
            UnitKind::Library,
        );
        assert_eq!(unit.name().as_str(), "u3-event-store");
        assert_eq!(unit.kind(), UnitKind::Library);
    }
}
