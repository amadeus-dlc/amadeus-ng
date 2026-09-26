//! `SteeringPartRow` — `read_steering_part` の 1 行 (配信計画の 1 部の中身)。

/// `read_steering_part` の 1 行。主キーは 1 列 `id` (自然キー (`phase`, `part_index`) から
/// 導いた代理キー)。`steering_plan_id` は `read_steering_plan.id` を指す FK である。
///
/// `part_index` は **1 始まり**である (upstream の部番号と同じ数え方 — 「1 / 3 部」)。
/// `rules_content` は `[{path, text}]` の 1 行 JSON で、`load-steering` が届ける中身
/// そのものである。
///
/// [`super::SteeringPlanRow`] と同じく `as_of` 列を持たない (参照入力由来)。行は値を運ぶ
/// だけである。規則束から行を組む投影は [`crate::read_tables::SteeringTables::pack`] が持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteeringPartRow {
    id: String,
    steering_plan_id: String,
    phase: String,
    part_index: usize,
    rules_content: String,
}

impl SteeringPartRow {
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        id: String,
        steering_plan_id: String,
        phase: String,
        part_index: usize,
        rules_content: String,
    ) -> Self {
        Self {
            id,
            steering_plan_id,
            phase,
            part_index,
            rules_content,
        }
    }

    /// 主キー — 自然キー (`phase`, `part_index`) から導いた代理キー。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// `read_steering_plan.id` を指す FK (同じフェーズの配信計画)。
    #[must_use]
    pub fn steering_plan_id(&self) -> &str {
        &self.steering_plan_id
    }

    /// フェーズの綴り (`PhaseId::as_str`)。
    #[must_use]
    pub fn phase(&self) -> &str {
        &self.phase
    }

    /// 部の番号 (1 始まり)。
    #[must_use]
    pub const fn part_index(&self) -> usize {
        self.part_index
    }

    /// この部が届ける `[{path, text}]` の 1 行 JSON 配列。
    #[must_use]
    pub fn rules_content(&self) -> &str {
        &self.rules_content
    }
}
