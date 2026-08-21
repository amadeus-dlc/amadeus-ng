//! `SkeletonStance` — on / off / scope-dependent (10 §2.2)。エンジンが計算できない唯一の
//! ゲート値で、conductor の分類が `report --skeleton-stance` で返る。

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkeletonStance {
    On,
    Off,
    ScopeDependent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownStance(String);

impl UnknownStance {
    #[must_use]
    pub fn new(value: impl Into<String>) -> UnknownStance {
        UnknownStance(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl SkeletonStance {
    /// # Errors
    ///
    /// 3 値以外は `UnknownStance` で拒否する。
    pub fn parse(s: &str) -> Result<SkeletonStance, UnknownStance> {
        Ok(match s {
            "on" => SkeletonStance::On,
            "off" => SkeletonStance::Off,
            "scope-dependent" => SkeletonStance::ScopeDependent,
            other => return Err(UnknownStance::new(other)),
        })
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            SkeletonStance::On => "on",
            SkeletonStance::Off => "off",
            SkeletonStance::ScopeDependent => "scope-dependent",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_values_round_trip_and_unknown_is_rejected() {
        for s in ["on", "off", "scope-dependent"] {
            assert_eq!(SkeletonStance::parse(s).unwrap().as_str(), s);
        }
        // 3 値以外は生値を逐語で持ち帰る
        let rejected = SkeletonStance::parse("maybe").unwrap_err();
        assert_eq!(rejected.as_str(), "maybe");
        assert_eq!(rejected, UnknownStance::new("maybe"));
    }
}
