//! `ResolvedPlan` が保持する計画上の 1 ステージ。

use core_command_domain::orchestration::StageDisplay;
use core_command_domain::workflow_definition::{PhaseId, PlanAction, StageSlug};

/// 計画上の 1 ステージ（`StageEntry` から投影が要る分だけ写したもの）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedStage {
    slug: StageSlug,
    phase: PhaseId,
    plan_action: PlanAction,
    display: StageDisplay,
}

impl PlannedStage {
    /// `intent` の誕生記録から投影が要る分だけ写す（`ResolvedPlan::of` 専用）。
    pub(super) fn from_stage_entry(
        entry: &core_command_domain::orchestration::StageEntry,
    ) -> PlannedStage {
        PlannedStage {
            slug: entry.slug().clone(),
            phase: entry.phase(),
            plan_action: entry.plan_action(),
            display: entry.display().clone(),
        }
    }

    /// ステージ slug。
    #[must_use]
    pub const fn slug(&self) -> &StageSlug {
        &self.slug
    }

    /// このステージのフェーズ。
    #[must_use]
    pub const fn phase(&self) -> PhaseId {
        self.phase
    }

    /// 静的グリッド由来の計画。
    #[must_use]
    pub const fn plan_action(&self) -> PlanAction {
        self.plan_action
    }

    /// 表示属性（番号・表題・担当）。
    #[must_use]
    pub const fn display(&self) -> &StageDisplay {
        &self.display
    }

    /// スコープ内か（`EXECUTE` のもの）。
    #[must_use]
    pub fn is_in_scope(&self) -> bool {
        self.plan_action == PlanAction::Execute
    }
}
