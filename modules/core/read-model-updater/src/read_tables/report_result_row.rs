//! 保存された報告の結果行。現在状態からの導出は行わない。
use super::json_column;
use core_command_domain::orchestration::{ReportNoOp, ReportResult, Reported};

/// 呼出側の報告識別子で一意に引く行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportResultRow {
    report_id: String,
    execution_id: String,
    stage: String,
    scope: String,
    result_kind: String,
    steps: String,
    no_op_reason: Option<String>,
    current_stage: Option<String>,
}
impl ReportResultRow {
    /// イベントに属する事実を読取り表へ写す。
    #[must_use]
    pub fn of(reported: &Reported) -> Self {
        let (stage, scope, result_kind, steps, no_op_reason, current_stage) =
            match reported.result() {
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
        Self {
            report_id: reported.report_id().to_string(),
            execution_id: reported.aggregate_id().to_string(),
            stage: stage.to_string(),
            scope: scope.clone(),
            result_kind: result_kind.to_string(),
            steps: json_column::strings(&steps),
            no_op_reason,
            current_stage,
        }
    }
    /// SQL列へ渡すreport_id。
    #[must_use]
    pub fn report_id(&self) -> &str {
        &self.report_id
    }
    /// SQL列へ渡すexecution_id。
    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }
    /// SQL列へ渡すstage。
    #[must_use]
    pub fn stage(&self) -> &str {
        &self.stage
    }
    /// SQL列へ渡すscope。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }
    /// SQL列へ渡すresult_kind。
    #[must_use]
    pub fn result_kind(&self) -> &str {
        &self.result_kind
    }
    /// SQL列へ渡すsteps。
    #[must_use]
    pub fn steps(&self) -> &str {
        &self.steps
    }
    /// SQL列へ渡すno_op_reason。
    #[must_use]
    pub fn no_op_reason(&self) -> Option<&str> {
        self.no_op_reason.as_deref()
    }
    /// SQL列へ渡すcurrent_stage。
    #[must_use]
    pub fn current_stage(&self) -> Option<&str> {
        self.current_stage.as_deref()
    }
}
