//! `StageSkipped` の永続化 DTO (**読む側**)。

use core_command_domain::orchestration::StageSkipped;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::intent_execution_event_dto::{aggregate_id_of, event_id_of, slug_of, slug_spelling};

/// `StageSkipped` の材料。**`id` (イベント自身の識別子) と `aggregate_id`
/// (どの集約の事実か) を先頭に置く並びが契約**である。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageSkippedDto {
    id: String,
    aggregate_id: String,
    stage: String,
    reason: String,
}

impl StageSkippedDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(super) fn of(payload: &StageSkipped) -> StageSkippedDto {
        StageSkippedDto {
            id: payload.id().as_str().to_string(),
            aggregate_id: payload.aggregate_id().as_str().to_string(),
            stage: slug_spelling(payload.stage()),
            reason: payload.reason().to_string(),
        }
    }

    /// ドメインの材料へ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<StageSkipped, DtoDecodeError> {
        Ok(StageSkipped::new(
            event_id_of(&self.id)?,
            aggregate_id_of(&self.aggregate_id)?,
            slug_of(&self.stage, "stage")?,
            self.reason.clone(),
        ))
    }
}
