//! 受理した回答の保存表現。
use super::dto_decode_error::DtoDecodeError;
use core_command_domain::orchestration::{
    AnswerDisposition, AnswerId, AnswerRecorded, IntentExecutionEventId, IntentExecutionId,
};
use serde::{Deserialize, Serialize};
/// AnswerRecordedの境界DTO。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerRecordedDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    summary: Option<SummaryEvidenceDto>,
    id: String,
    aggregate_id: String,
    answer_id: String,
    stage: String,
    details: String,
    disposition: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SummaryEvidenceDto {
    questions_file: String,
    questions_sha256: String,
}

impl AnswerRecordedDto {
    pub(super) fn of(event: &AnswerRecorded) -> Self {
        Self {
            summary: match event.disposition() {
                AnswerDisposition::SummaryConfirmed(evidence) => Some(SummaryEvidenceDto {
                    questions_file: evidence.questions_file().to_string(),
                    questions_sha256: evidence.questions_sha256().to_string(),
                }),
                _ => None,
            },
            id: event.id().as_str().to_string(),
            aggregate_id: event.aggregate_id().as_str().to_string(),
            answer_id: event.answer_id().as_str().to_string(),
            stage: event.stage().to_string(),
            details: event.details().to_string(),
            disposition: match event.disposition() {
                AnswerDisposition::SummaryConfirmed(_) => "summary-confirmation",
                AnswerDisposition::Recorded => "recorded",
                AnswerDisposition::ApprovalGateReportOwned => "approval-gate-report-owned",
            }
            .to_string(),
        }
    }
    pub(super) fn to_domain(&self) -> Result<AnswerRecorded, DtoDecodeError> {
        Ok(AnswerRecorded::new(
            IntentExecutionEventId::parse(&self.id)
                .map_err(|_| DtoDecodeError::malformed("id", &self.id))?,
            IntentExecutionId::parse(&self.aggregate_id)
                .map_err(|_| DtoDecodeError::malformed("aggregate_id", &self.aggregate_id))?,
            AnswerId::parse(&self.answer_id)
                .map_err(|_| DtoDecodeError::malformed("answer_id", &self.answer_id))?,
            &self.stage,
            &self.details,
            match self.disposition.as_str() {
                "summary-confirmation" => {
                    let summary = self
                        .summary
                        .as_ref()
                        .ok_or_else(|| DtoDecodeError::malformed("summary", "missing"))?;
                    AnswerDisposition::SummaryConfirmed(
                        core_command_domain::orchestration::SummaryEvidence::new(
                            &summary.questions_file,
                            &summary.questions_sha256,
                        )
                        .map_err(|error| DtoDecodeError::malformed("summary", error.to_string()))?,
                    )
                }
                "recorded" => AnswerDisposition::Recorded,
                "approval-gate-report-owned" => AnswerDisposition::ApprovalGateReportOwned,
                _ => return Err(DtoDecodeError::malformed("disposition", &self.disposition)),
            },
        ))
    }
}
