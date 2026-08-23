//! `PhaseBoundary` — `GateApproved` に載るフェーズ境界の投影材料 (C5)。

use crate::workflow_definition::PhaseId;

/// 承認によって跨いだフェーズ境界。
///
/// **呼出側 (ユースケース) が定義から導出して渡す投影材料**であり、集約は検証せず
/// イベントに載せるだけである (functional-spec §2)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhaseBoundary {
    from_phase: PhaseId,
    to_phase: PhaseId,
}

impl PhaseBoundary {
    /// 境界の両端をそのまま束ねる (検証しない)。
    #[must_use]
    pub const fn new(from_phase: PhaseId, to_phase: PhaseId) -> PhaseBoundary {
        PhaseBoundary {
            from_phase,
            to_phase,
        }
    }

    /// 完了した側のフェーズ。
    #[must_use]
    pub const fn from_phase(self) -> PhaseId {
        self.from_phase
    }

    /// 開始する側のフェーズ。
    #[must_use]
    pub const fn to_phase(self) -> PhaseId {
        self.to_phase
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_definition::PhaseId;

    #[test]
    fn the_boundary_carries_both_ends() {
        let b = PhaseBoundary::new(PhaseId::Inception, PhaseId::Construction);
        assert_eq!(b.from_phase(), PhaseId::Inception);
        assert_eq!(b.to_phase(), PhaseId::Construction);
    }

    #[test]
    fn boundaries_compare_by_value() {
        let a = PhaseBoundary::new(PhaseId::Ideation, PhaseId::Inception);
        assert_eq!(a, PhaseBoundary::new(PhaseId::Ideation, PhaseId::Inception));
        assert_ne!(a, PhaseBoundary::new(PhaseId::Ideation, PhaseId::Operation));
    }

    #[test]
    fn a_boundary_may_name_the_same_phase_twice() {
        // 集約は検証しない (呼出側が導出して渡す投影材料 — BR2.4)。
        let b = PhaseBoundary::new(PhaseId::Construction, PhaseId::Construction);
        assert_eq!(b.from_phase(), b.to_phase());
    }
}
