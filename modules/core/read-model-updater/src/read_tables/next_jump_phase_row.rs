//! `NextJumpPhaseRow` — `read_next_jump_phase` の 1 行 (`--phase` ジャンプの目的地)。

use core_command_domain::orchestration::{IntentExecution, StageIndex};
use core_command_domain::workflow_definition::PhaseId;

use super::stage_lookup::slug_of;

/// `read_next_jump_phase` の 1 行。主キーは (`execution_id`, `phase`)。
///
/// 値は集約のクエリ [`IntentExecution::first_in_scope_of_phase`] の答えである。
/// 目的地は**実効プラン**で決まる (recompose のオーバレイが静的グリッドに勝つ) ので、
/// 定義側の `read_definition_scope_phase_entry` とは答えが違いうる — 2 つの表は別の
/// 理由で変わる。
///
/// 答えが `None` のフェーズには行を作らない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextJumpPhaseRow {
    execution_id: String,
    phase: String,
    target_index: usize,
    target_slug: Option<String>,
}

impl NextJumpPhaseRow {
    /// 1 つのフェーズの目的地を 1 行へ写す (**この型の唯一の構築経路**)。
    #[must_use]
    pub fn of(execution: &IntentExecution, phase: PhaseId, target: StageIndex) -> NextJumpPhaseRow {
        NextJumpPhaseRow {
            execution_id: execution.id().as_str().to_string(),
            phase: phase.as_str().to_string(),
            target_index: target.to_usize(),
            target_slug: slug_of(execution, target.to_usize()),
        }
    }

    /// 実行の識別子。
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
