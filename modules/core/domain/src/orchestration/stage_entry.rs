//! `StageEntry` — `Started` に載る解決済みの 1 ステージ分の計画 (entities.md StageEntry)。

use crate::workflow_definition::{PhaseId, PlanAction, StageSlug};

/// 定義から解決済みの 1 ステージ分の計画。
///
/// `Started` がこの列を持つことでリプレイは `WorkflowDefinition` を要さない (BR2.2)。
/// ゲート判定はこの型が所有する — 索引ではなく `phase` から決まる (BR1.3、Tell-Don't-Ask)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageEntry {
    slug: StageSlug,
    phase: PhaseId,
    plan_action: PlanAction,
    conditional: bool,
}

impl StageEntry {
    /// 解決済みの 4 成分を束ねる。
    ///
    /// `plan_action` はグリッドの 3 値 `Option<PlanAction>` を `None → SKIP` で畳んだ 2 値、
    /// `conditional` は同じ文書順の `StageNode::execution() == CONDITIONAL` (BR2.2)。
    #[must_use]
    pub const fn new(
        slug: StageSlug,
        phase: PhaseId,
        plan_action: PlanAction,
        conditional: bool,
    ) -> StageEntry {
        StageEntry {
            slug,
            phase,
            plan_action,
            conditional,
        }
    }

    /// ステージ slug (イベントのステージ参照はすべてこの値)。
    #[must_use]
    pub const fn slug(&self) -> &StageSlug {
        &self.slug
    }

    /// このステージのフェーズ。
    #[must_use]
    pub const fn phase(&self) -> PhaseId {
        self.phase
    }

    /// 静的グリッド由来の計画 (`plan`)。recompose オーバレイはここには載らない。
    #[must_use]
    pub const fn plan_action(&self) -> PlanAction {
        self.plan_action
    }

    /// ステージ著者側の適用可否が CONDITIONAL か。
    #[must_use]
    pub const fn is_conditional(&self) -> bool {
        self.conditional
    }

    /// ゲート付きか — `phase != initialization` (BR1.3)。索引 0 の特別扱いはしない。
    #[must_use]
    pub fn is_gated(&self) -> bool {
        self.phase != PhaseId::Initialization
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_definition::{PhaseId, PlanAction, StageSlug};

    fn entry(phase: PhaseId, action: PlanAction, conditional: bool) -> StageEntry {
        StageEntry::new(
            StageSlug::parse("state-init").unwrap(),
            phase,
            action,
            conditional,
        )
    }

    #[test]
    fn the_entry_carries_the_resolved_plan_of_one_stage() {
        let e = entry(PhaseId::Inception, PlanAction::Execute, true);
        assert_eq!(e.slug().as_str(), "state-init");
        assert_eq!(e.phase(), PhaseId::Inception);
        assert_eq!(e.plan_action(), PlanAction::Execute);
        assert!(e.is_conditional());
    }

    #[test]
    fn an_initialization_stage_is_not_gated() {
        let e = entry(PhaseId::Initialization, PlanAction::Execute, false);
        assert!(!e.is_gated());
    }

    #[test]
    fn every_other_phase_is_gated() {
        for phase in [
            PhaseId::Ideation,
            PhaseId::Inception,
            PhaseId::Construction,
            PhaseId::Operation,
        ] {
            assert!(
                entry(phase, PlanAction::Execute, false).is_gated(),
                "{phase:?}"
            );
        }
    }

    #[test]
    fn an_unconditional_entry_reports_it() {
        assert!(!entry(PhaseId::Inception, PlanAction::Skip, false).is_conditional());
    }

    #[test]
    fn entries_compare_by_value() {
        let a = entry(PhaseId::Inception, PlanAction::Execute, false);
        let b = entry(PhaseId::Inception, PlanAction::Execute, false);
        let c = entry(PhaseId::Inception, PlanAction::Skip, false);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
