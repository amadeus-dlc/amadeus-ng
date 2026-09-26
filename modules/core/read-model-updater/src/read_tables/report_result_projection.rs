//! `read_report_result` の行を組む投影 — 保存された報告を [`ReportResultRow`] へ写す。
//! 現在状態からの導出は行わない。

use core_command_domain::orchestration::{ReportNoOp, ReportResult, Reported};

use super::json_column;
use crate::orchestration::ReportResultRow;

/// イベントに属する事実を読取り表へ写す。
pub(super) fn row(reported: &Reported) -> ReportResultRow {
    let (stage, scope, result_kind, steps, no_op_reason, current_stage) = match reported.result() {
        ReportResult::Committed {
            stage,
            scope,
            steps,
            ..
        } => (
            stage,
            scope,
            "committed",
            steps.fold_left(Vec::new(), |mut values, step| {
                values.push(step.subcommand().to_string());
                values
            }),
            None,
            None,
        ),
        ReportResult::NoOp { scope, no_op } => {
            let (stage, reason, current) = match no_op {
                ReportNoOp::AlreadyAwaiting { stage } => (stage, "already_awaiting", None),
                ReportNoOp::AlreadyCompletedMovedOn { stage, current } => (
                    stage,
                    "already_completed_moved_on",
                    Some(current.to_string()),
                ),
                ReportNoOp::WorkflowAlreadyCompleted { stage } => {
                    (stage, "workflow_already_completed", None)
                }
            };
            (
                stage,
                scope,
                "no_op",
                Vec::new(),
                Some(reason.to_string()),
                current,
            )
        }
    };
    ReportResultRow::new(
        reported.report_id().to_string(),
        reported.aggregate_id().to_string(),
        stage.to_string(),
        scope.clone(),
        result_kind.to_string(),
        json_column::strings(&steps),
        no_op_reason,
        current_stage,
    )
}
