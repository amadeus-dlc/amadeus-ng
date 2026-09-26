//! `read_execution` の行を組む投影 — 実行の集約を [`ExecutionRow`] へ写す。

use chrono::SecondsFormat;
use core_command_domain::orchestration::{Intent, IntentExecution, SkeletonStance, StageIndex};

use super::spelling;
use super::stage_lookup::slug_at;
use crate::orchestration::ExecutionRow;

/// 実行の集約を 1 行へ写す。
///
/// `scope` だけは実行が持たない — 選ばれた scope は静的な intent の持ち物なので、
/// この実行が指す intent から**非正規化して**載せる。読取コマンドは scope で分岐する
/// たびに intent の行を引き直さずに済む (裁定 §10-1)。
pub(super) fn row(execution: &IntentExecution, intent: &Intent) -> ExecutionRow {
    let cursor = execution.cursor();
    ExecutionRow::new(
        execution.id().as_str().to_string(),
        execution.is_first_substantive_run(),
        execution.continuation_wait().map(|reason| {
            match reason {
                core_command_domain::orchestration::ContinuationWait::GateOrRevision => {
                    "gate-or-revision"
                }
                core_command_domain::orchestration::ContinuationWait::Decision => "decision",
                core_command_domain::orchestration::ContinuationWait::Question => "question",
                core_command_domain::orchestration::ContinuationWait::Conversation => {
                    "conversation"
                }
                core_command_domain::orchestration::ContinuationWait::Resume => "resume",
            }
            .to_string()
        }),
        execution.intent_id().as_str().to_string(),
        intent.scope().to_string(),
        spelling::status(execution.status()).to_string(),
        Some(cursor.to_usize()),
        slug_at(execution, cursor),
        execution.parked_at().map(StageIndex::to_usize),
        execution.parked_at().and_then(|at| slug_at(execution, at)),
        execution.parked_active(),
        execution.accepts_commands(),
        execution.autonomy().as_state_field().to_string(),
        execution
            .skeleton_stance()
            .map(|stance| SkeletonStance::as_str(stance).to_string()),
        execution.seq_nr(),
        execution
            .last_updated_at()
            .to_rfc3339_opts(SecondsFormat::Secs, true),
        execution.state_binding().as_str().to_string(),
    )
}
