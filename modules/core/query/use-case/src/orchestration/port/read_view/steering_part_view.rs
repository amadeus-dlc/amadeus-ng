//! `SteeringPartView` — `read_steering_part` 1 行の写し (配信計画の 1 部)。

/// `read_steering_part` の 1 行 (自然キー `phase` × `part_index` は UNIQUE 索引)。
///
/// `part_index` は **1 始まり**である (upstream の部番号と同じ数え方 — 「1 / 3 部」)。
/// [`SteeringPartView::steering_plan_id`] は所属する計画を指す FK であり、この行を引く
/// 鍵でもある。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteeringPartView {
    steering_plan_id: String,
    phase: String,
    part_index: u32,
    rules_content: String,
}

impl SteeringPartView {
    /// 4 列をそのまま束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        steering_plan_id: String,
        phase: String,
        part_index: u32,
        rules_content: String,
    ) -> SteeringPartView {
        SteeringPartView {
            steering_plan_id,
            phase,
            part_index,
            rules_content,
        }
    }

    /// 所属する配信計画を指す FK。
    #[must_use]
    pub fn steering_plan_id(&self) -> &str {
        &self.steering_plan_id
    }

    /// フェーズの綴り。
    #[must_use]
    pub fn phase(&self) -> &str {
        &self.phase
    }

    /// 部の番号 (1 始まり)。
    #[must_use]
    pub const fn part_index(&self) -> u32 {
        self.part_index
    }

    /// この部が届ける `[{path, text}]` の 1 行 JSON 配列。
    #[must_use]
    pub fn rules_content(&self) -> &str {
        &self.rules_content
    }
}
