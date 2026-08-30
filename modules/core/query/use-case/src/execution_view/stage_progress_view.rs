//! `StageProgressView` — Stage Progress 1 行分のクエリモデル。
//!
//! リードモデル `aidlc-state.md` の `### <PHASE> PHASE` 見出し配下にある
//! `- [<marker>] <slug> — <EXECUTE|SKIP>` 行 1 本を写す。旧コマンド側では 3 つの型
//! (`StageEntry` の静的計画・`StageKey` の適用添字帳・集約の実行時ベクトル) に分かれていた
//! 材料が、リードモデル上では**この 1 行に畳まれている**ため、クエリ側は 1 型で持つ。

use super::checkbox_state::CheckboxState;
use crate::workflow_view::{PhaseView, PlanActionView, StageSlugView};

/// 1 ステージ分の進捗行 (構築後 immutable)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageProgressView {
    slug: StageSlugView,
    phase: PhaseView,
    checkbox: CheckboxState,
    plan: PlanActionView,
}

impl StageProgressView {
    /// 行の 4 成分を束ねる。
    ///
    /// `phase` は行が属する `### <PHASE> PHASE` 見出し、`plan` は行末の EXECUTE / SKIP
    /// トークン (**実効**プラン — recompose のオーバレイは投影が既に行末へ書き戻している)。
    #[must_use]
    pub const fn new(
        slug: StageSlugView,
        phase: PhaseView,
        checkbox: CheckboxState,
        plan: PlanActionView,
    ) -> StageProgressView {
        StageProgressView {
            slug,
            phase,
            checkbox,
            plan,
        }
    }

    /// ステージ slug (行の識別子)。
    #[must_use]
    pub const fn slug(&self) -> &StageSlugView {
        &self.slug
    }

    /// 行が属するフェーズ。
    #[must_use]
    pub const fn phase(&self) -> PhaseView {
        self.phase
    }

    /// checkbox マーカーが表す run-state。
    #[must_use]
    pub const fn checkbox(&self) -> CheckboxState {
        self.checkbox
    }

    /// 行末トークンが表す実効プラン。
    #[must_use]
    pub const fn plan(&self) -> PlanActionView {
        self.plan
    }

    /// スコープ内か — 実効プランが EXECUTE (BR4.2 の `in_scope`)。
    #[must_use]
    pub fn is_in_scope(&self) -> bool {
        self.plan == PlanActionView::Execute
    }

    /// ゲート付きか — `phase != initialization` (BR1.3)。索引 0 の特別扱いはしない。
    #[must_use]
    pub fn is_gated(&self) -> bool {
        self.phase != PhaseView::Initialization
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(phase: PhaseView, plan: PlanActionView) -> StageProgressView {
        StageProgressView::new(
            StageSlugView::parse("state-init").unwrap(),
            phase,
            CheckboxState::Pending,
            plan,
        )
    }

    #[test]
    fn the_row_reports_its_four_faces() {
        let entry = StageProgressView::new(
            StageSlugView::parse("functional-design").unwrap(),
            PhaseView::Construction,
            CheckboxState::InProgress,
            PlanActionView::Execute,
        );
        assert_eq!(entry.slug().as_str(), "functional-design");
        assert_eq!(entry.phase(), PhaseView::Construction);
        assert_eq!(entry.checkbox(), CheckboxState::InProgress);
        assert_eq!(entry.plan(), PlanActionView::Execute);
    }

    #[test]
    fn in_scope_is_the_execute_token() {
        assert!(row(PhaseView::Inception, PlanActionView::Execute).is_in_scope());
        assert!(!row(PhaseView::Inception, PlanActionView::Skip).is_in_scope());
    }

    #[test]
    fn gating_follows_the_phase_not_the_position() {
        assert!(!row(PhaseView::Initialization, PlanActionView::Execute).is_gated());
        for phase in [
            PhaseView::Ideation,
            PhaseView::Inception,
            PhaseView::Construction,
            PhaseView::Operation,
        ] {
            assert!(row(phase, PlanActionView::Execute).is_gated());
        }
    }
}
