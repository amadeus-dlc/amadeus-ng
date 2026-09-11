//! 学びの記録の永続化表現。ドメインとは独立したDTO。
use super::dto_decode_error::DtoDecodeError;
use core_command_domain::orchestration::{
    CapturedLearning, CapturedLearnings, IntentExecutionEventId, IntentExecutionId, Learning,
    LearningCandidateId, LearningDisposition, LearningProvenance, LearningScope, LearningSource,
    LearningsCaptured, PracticeHeading,
};
use core_command_domain::workflow_definition::StageSlug;
use core_command_domain::workspace::{IntentDirName, SpaceName};
use serde::{Deserialize, Serialize};

/// 書込みの内訳の綴り (この DTO の内部だけで使う)。
const FRESH: &str = "fresh";
/// 監査行は在るが実践行が無い。
const PRACTICE_LINE_ONLY: &str = "practice_line_only";
/// 実践行は在るが監査行が無い。
const AUDIT_ROW_ONLY: &str = "audit_row_only";

/// 1 件の学びの行 (この DTO の内部だけで使う)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CapturedLearningDto {
    candidate_id: String,
    scope: String,
    heading: String,
    text: String,
    source: String,
    disposition: String,
}

/// 保存側と読取側がそれぞれ所有する学びの記録の表現。
///
/// 同一性 (`content_hash`) は本文から導けるので列に持たない — 保存した綴りと導出が食い違う
/// 余地を構造から無くす。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearningsCapturedDto {
    id: String,
    aggregate_id: String,
    stage: String,
    space: String,
    intent: String,
    learnings: Vec<CapturedLearningDto>,
}

/// 内訳の綴り。
const fn disposition_spelling(disposition: LearningDisposition) -> &'static str {
    match disposition {
        LearningDisposition::Fresh => FRESH,
        LearningDisposition::PracticeLineOnly => PRACTICE_LINE_ONLY,
        LearningDisposition::AuditRowOnly => AUDIT_ROW_ONLY,
    }
}

/// 綴りから内訳へ。未知の綴りは復号失敗である (成功に丸めない)。
fn disposition_of(spelling: &str) -> Option<LearningDisposition> {
    match spelling {
        FRESH => Some(LearningDisposition::Fresh),
        PRACTICE_LINE_ONLY => Some(LearningDisposition::PracticeLineOnly),
        AUDIT_ROW_ONLY => Some(LearningDisposition::AuditRowOnly),
        _ => None,
    }
}

impl LearningsCapturedDto {
    pub(super) fn of(event: &LearningsCaptured) -> Self {
        Self {
            id: event.id().as_str().to_string(),
            aggregate_id: event.aggregate_id().as_str().to_string(),
            stage: event.stage().as_str().to_string(),
            space: event.provenance().space().as_str().to_string(),
            intent: event.provenance().intent().as_str().to_string(),
            learnings: event
                .learnings()
                .fold_left(Vec::new(), |mut rows, captured| {
                    let learning = captured.learning();
                    rows.push(CapturedLearningDto {
                        candidate_id: learning.candidate_id().as_str().to_string(),
                        scope: learning.scope().as_str().to_string(),
                        heading: learning.heading().as_str().to_string(),
                        text: learning.text().to_string(),
                        source: learning.source().as_str().to_string(),
                        disposition: disposition_spelling(captured.disposition()).to_string(),
                    });
                    rows
                }),
        }
    }

    pub(super) fn to_domain(&self) -> Result<Box<LearningsCaptured>, DtoDecodeError> {
        let mut learnings = Vec::with_capacity(self.learnings.len());
        for row in &self.learnings {
            let disposition = disposition_of(&row.disposition).ok_or_else(|| {
                DtoDecodeError::malformed("learnings.disposition", &row.disposition)
            })?;
            learnings.push(CapturedLearning::new(
                Learning::new(
                    LearningCandidateId::parse(&row.candidate_id).map_err(|_| {
                        DtoDecodeError::malformed("learnings.candidate_id", &row.candidate_id)
                    })?,
                    LearningScope::of_spelling(&row.scope),
                    PracticeHeading::from_routed(&row.heading),
                    row.text.clone(),
                    LearningSource::of_spelling(&row.source),
                ),
                disposition,
            ));
        }
        Ok(Box::new(LearningsCaptured::new(
            IntentExecutionEventId::parse(&self.id)
                .map_err(|_| DtoDecodeError::malformed("id", &self.id))?,
            IntentExecutionId::parse(&self.aggregate_id)
                .map_err(|_| DtoDecodeError::malformed("aggregate_id", &self.aggregate_id))?,
            StageSlug::parse(&self.stage)
                .map_err(|_| DtoDecodeError::malformed("stage", &self.stage))?,
            LearningProvenance::new(
                SpaceName::parse(&self.space)
                    .map_err(|_| DtoDecodeError::malformed("space", &self.space))?,
                IntentDirName::parse(&self.intent)
                    .map_err(|_| DtoDecodeError::malformed("intent", &self.intent))?,
            ),
            CapturedLearnings::new(learnings),
        )))
    }
}
