//! `SteeringDeliveryView` — 配信計画とその第 1 部 (ユースケースが FK をたどって組む)。

use crate::orchestration::{SteeringPartView, SteeringPlanView};

/// `read_steering_plan` の 1 行と、その計画の第 1 部 (`read_steering_part`)。
///
/// # 行の写しを埋め込む — 列を写し直さない
///
/// 両面とも引いた表の行そのものである。空計画 (部の行が無い) は `first_part` が `None` に
/// なるが、これは**行の有無をそのまま伝播した**ものであって判断ではない。
///
/// [`NextTurnView`](super::NextTurnView) の `plan` / `first_part` と同じ 2 面を、答えの行を
/// 経ずに run-stage の FK から直接たどったときの形である (`--single` と state なし jump は
/// 答えの行を持たない — 実行がまだ無いか、要求が実行を経由しない)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteeringDeliveryView {
    plan: SteeringPlanView,
    first_part: Option<SteeringPartView>,
}

impl SteeringDeliveryView {
    /// 2 段の引当結果を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        plan: SteeringPlanView,
        first_part: Option<SteeringPartView>,
    ) -> SteeringDeliveryView {
        SteeringDeliveryView { plan, first_part }
    }

    /// 配信計画の行。
    #[must_use]
    pub const fn plan(&self) -> &SteeringPlanView {
        &self.plan
    }

    /// 計画の 1 部目 (空計画なら `None`)。
    #[must_use]
    pub const fn first_part(&self) -> Option<&SteeringPartView> {
        self.first_part.as_ref()
    }
}
