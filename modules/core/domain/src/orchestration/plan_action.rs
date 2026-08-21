//! `PlanAction` — EXECUTE / SKIP (01 §3.1)。scope grid の列値であり、recompose オーバレイと
//! 合成した実効プラン (`effectivePlanAction` — 裁定 B1) の要素。

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlanAction {
    Execute,
    Skip,
}

impl PlanAction {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            PlanAction::Execute => "EXECUTE",
            PlanAction::Skip => "SKIP",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<PlanAction> {
        match s {
            "EXECUTE" => Some(PlanAction::Execute),
            "SKIP" => Some(PlanAction::Skip),
            _ => None,
        }
    }

    #[must_use]
    pub const fn flipped(self) -> PlanAction {
        match self {
            PlanAction::Execute => PlanAction::Skip,
            PlanAction::Skip => PlanAction::Execute,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_rejects_lowercase() {
        assert_eq!(PlanAction::parse("EXECUTE"), Some(PlanAction::Execute));
        assert_eq!(PlanAction::parse("SKIP"), Some(PlanAction::Skip));
        assert_eq!(PlanAction::parse("execute"), None);
    }
}
