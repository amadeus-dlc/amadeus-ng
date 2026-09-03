//! `PhaseEntryView` — `read_definition_scope_phase_entry` 1 行の写し。

/// `read_definition_scope_phase_entry` の 1 行 (主キー `definition_id` × `scope` × `phase`)。
///
/// 定義とスコープグリッドだけで決まる「そのフェーズの入口」である。実行の実効プランで
/// 決まる入口は `read_next_jump_phase` が別に持つ — 2 つの表は別の理由で変わる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseEntryView {
    first_stage_slug: String,
}

impl PhaseEntryView {
    /// 入口ステージの slug を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(first_stage_slug: String) -> PhaseEntryView {
        PhaseEntryView { first_stage_slug }
    }

    /// そのフェーズで最初に実行するステージの slug。
    #[must_use]
    pub fn first_stage_slug(&self) -> &str {
        &self.first_stage_slug
    }
}
