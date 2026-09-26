//! `NextJumpPhaseRow` — `read_next_jump_phase` の 1 行 (`--phase` ジャンプの目的地)。

/// `read_next_jump_phase` の 1 行。主キーは 1 列 `id` (自然キー
/// (`execution_id`, `phase`) から導いた代理キー)。`execution_id` は `read_execution.id` を
/// 指す FK である。
///
/// 値は集約のクエリ [`IntentExecution::first_in_scope_of_phase`] の答えである。
/// 目的地は**実効プラン**で決まる (recompose のオーバレイが静的グリッドに勝つ) ので、
/// 定義側の `read_definition_scope_phase_entry` とは答えが違いうる — 2 つの表は別の
/// 理由で変わる。
///
/// 答えが `None` のフェーズには行を作らない。
///
/// 行は値を運ぶだけである。材料から行を組む投影は
/// [`crate::read_tables::ReadTables::project`] が持つ。
///
/// [`IntentExecution::first_in_scope_of_phase`]: core_command_domain::orchestration::IntentExecution::first_in_scope_of_phase
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextJumpPhaseRow {
    id: String,
    execution_id: String,
    phase: String,
    target_index: usize,
    target_slug: Option<String>,
}

impl NextJumpPhaseRow {
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        id: String,
        execution_id: String,
        phase: String,
        target_index: usize,
        target_slug: Option<String>,
    ) -> Self {
        Self {
            id,
            execution_id,
            phase,
            target_index,
            target_slug,
        }
    }

    /// 主キー — 自然キー (`execution_id`, `phase`) から導いた代理キー。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// `read_execution.id` を指す FK。
    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// フェーズの綴り (`PhaseId::as_str`)。
    #[must_use]
    pub fn phase(&self) -> &str {
        &self.phase
    }

    /// そのフェーズで最初に実行される in-scope ステージの位置。
    #[must_use]
    pub const fn target_index(&self) -> usize {
        self.target_index
    }

    /// その位置の slug。
    #[must_use]
    pub fn target_slug(&self) -> Option<&str> {
        self.target_slug.as_deref()
    }
}
