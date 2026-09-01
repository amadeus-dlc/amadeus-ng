//! `UnitKind` — units YAML の `kind` 閉集合。
//!
//! 語彙が閉じている (`service | spec | ui | packaging | library`) ので enum で運ぶ。綴りは
//! 公開言語であり、1 バイトも変えられない。

use super::unknown_unit_kind::UnknownUnitKind;

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
            other => Err(UnknownUnitKind::new(other)),
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
