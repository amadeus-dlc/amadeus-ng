//! `SteeringPlanView` — `read_steering_plan` 1 行の写し (1 フェーズの配信計画)。

/// `read_steering_plan` の 1 行 (自然キー `phase` は UNIQUE 索引)。
///
/// 計画そのものは「何部あるか」と「どのルールを配ったか」を言うだけで、部の中身は持たない
/// — 中身は `read_steering_part` の行であり、[`SteeringPlanView::id`] を鍵にして引く。
///
/// この表は参照入力 (人が編集する memory 層のルール) 由来なので、**ジャーナル由来 15 表とは
/// 別のトランザクション**で差し替わる。run-stage の FK が指す計画がまだ無いのは壊れた投影
/// ではなく正常な観測である (まだパックしていない)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteeringPlanView {
    id: String,
    phase: String,
    bundle_digest: String,
    part_count: u32,
    delivered_paths: String,
}

impl SteeringPlanView {
    /// 5 列をそのまま束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        id: String,
        phase: String,
        bundle_digest: String,
        part_count: u32,
        delivered_paths: String,
    ) -> SteeringPlanView {
        SteeringPlanView {
            id,
            phase,
            bundle_digest,
            part_count,
            delivered_paths,
        }
    }

    /// 主キー — `read_run_stage.steering_plan_id` と `read_steering_part.steering_plan_id`
    /// が指す値。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// フェーズの綴り。
    #[must_use]
    pub fn phase(&self) -> &str {
        &self.phase
    }

    /// ルール束のダイジェスト (`continue` の照合キー)。
    #[must_use]
    pub fn bundle_digest(&self) -> &str {
        &self.bundle_digest
    }

    /// パート総数 (0 = 空計画)。
    #[must_use]
    pub const fn part_count(&self) -> u32 {
        self.part_count
    }

    /// 配信済みルールのパス台帳の 1 行 JSON 配列。
    #[must_use]
    pub fn delivered_paths(&self) -> &str {
        &self.delivered_paths
    }
}
