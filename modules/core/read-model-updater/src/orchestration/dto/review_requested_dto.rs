//! `ReviewRequested` の永続化 DTO (**読む側**)。

use core_command_domain::orchestration::ReviewRequested;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::intent_execution_event_dto::{aggregate_id_of, event_id_of, slug_of, slug_spelling};

/// `ReviewRequested` の材料。**`id` (イベント自身の識別子) と `aggregate_id`
/// (どの集約の事実か) を先頭に置く並びが契約**である。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewRequestedDto {
    id: String,
    aggregate_id: String,
    stage: String,
    reviewer: String,
    iteration: u32,
    retry: bool,
}

impl ReviewRequestedDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(super) fn of(payload: &ReviewRequested) -> ReviewRequestedDto {
        ReviewRequestedDto {
            id: payload.id().as_str().to_string(),
            aggregate_id: payload.aggregate_id().as_str().to_string(),
            stage: slug_spelling(payload.stage()),
            reviewer: payload.reviewer().to_string(),
            iteration: payload.iteration(),
            retry: payload.is_retry(),
        }
    }

    /// ドメインの材料へ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<ReviewRequested, DtoDecodeError> {
        Ok(ReviewRequested::new(
            event_id_of(&self.id)?,
            aggregate_id_of(&self.aggregate_id)?,
            slug_of(&self.stage, "stage")?,
            self.reviewer.clone(),
            self.iteration,
            self.retry,
        ))
    }
}
