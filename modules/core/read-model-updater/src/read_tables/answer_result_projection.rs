//! `read_answer_result` の行を組む投影 — 受理された回答の事実を [`AnswerResultRow`] へ写す。

use core_command_domain::orchestration::{AnswerDisposition, AnswerRecorded};

use crate::orchestration::AnswerResultRow;

/// 保存された事実だけを写す。
pub(super) fn row(event: &AnswerRecorded) -> AnswerResultRow {
    AnswerResultRow::new(
        event.answer_id().as_str().to_string(),
        event.aggregate_id().as_str().to_string(),
        event.stage().to_string(),
        match event.disposition() {
            AnswerDisposition::SummaryConfirmed(_) => "summary-confirmation",
            AnswerDisposition::Recorded => "recorded",
            AnswerDisposition::ApprovalGateReportOwned => "approval-gate-report-owned",
        }
        .to_string(),
    )
}
