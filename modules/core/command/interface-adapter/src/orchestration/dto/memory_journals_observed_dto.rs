//! 日誌観測の永続化表現。ドメインとは独立したDTO。
use super::dto_decode_error::DtoDecodeError;
use core_command_domain::orchestration::{
    EmptyMemoryStages, IntentExecutionEventId, IntentExecutionId, MemoryJournal,
    MemoryJournalSurvey, MemoryJournalsObserved, StageMemoryJournal,
};
use core_command_domain::workflow_definition::StageSlug;
use serde::{Deserialize, Serialize};

/// 1 ステージ分の日誌観測の行 (この DTO の内部だけで使う)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StageMemoryJournalDto {
    stage: String,
    interpretations: u64,
    deviations: u64,
    tradeoffs: u64,
    open_questions: u64,
}

/// 保存側と読取側がそれぞれ所有する日誌観測の表現。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryJournalsObservedDto {
    id: String,
    aggregate_id: String,
    survey: Vec<StageMemoryJournalDto>,
    empty_stages: Vec<String>,
}

impl MemoryJournalsObservedDto {
    pub(super) fn of(event: &MemoryJournalsObserved) -> Self {
        Self {
            id: event.id().as_str().to_string(),
            aggregate_id: event.aggregate_id().as_str().to_string(),
            survey: event
                .survey()
                .fold_left(Vec::new(), |mut rows, observation| {
                    let journal = observation.journal();
                    rows.push(StageMemoryJournalDto {
                        stage: observation.stage().as_str().to_string(),
                        interpretations: journal.interpretations(),
                        deviations: journal.deviations(),
                        tradeoffs: journal.tradeoffs(),
                        open_questions: journal.open_questions(),
                    });
                    rows
                }),
            empty_stages: event
                .empty_stages()
                .fold_left(Vec::new(), |mut rows, slug| {
                    rows.push(slug.as_str().to_string());
                    rows
                }),
        }
    }

    pub(super) fn to_domain(&self) -> Result<Box<MemoryJournalsObserved>, DtoDecodeError> {
        let mut observations = Vec::with_capacity(self.survey.len());
        for row in &self.survey {
            observations.push(StageMemoryJournal::new(
                StageSlug::parse(&row.stage)
                    .map_err(|_| DtoDecodeError::malformed("survey.stage", &row.stage))?,
                MemoryJournal::new(
                    row.interpretations,
                    row.deviations,
                    row.tradeoffs,
                    row.open_questions,
                ),
            ));
        }
        let mut empty = Vec::with_capacity(self.empty_stages.len());
        for slug in &self.empty_stages {
            empty.push(
                StageSlug::parse(slug)
                    .map_err(|_| DtoDecodeError::malformed("empty_stages", slug))?,
            );
        }
        Ok(Box::new(MemoryJournalsObserved::new(
            IntentExecutionEventId::parse(&self.id)
                .map_err(|_| DtoDecodeError::malformed("id", &self.id))?,
            IntentExecutionId::parse(&self.aggregate_id)
                .map_err(|_| DtoDecodeError::malformed("aggregate_id", &self.aggregate_id))?,
            MemoryJournalSurvey::new(observations),
            EmptyMemoryStages::new(empty),
        )))
    }
}
