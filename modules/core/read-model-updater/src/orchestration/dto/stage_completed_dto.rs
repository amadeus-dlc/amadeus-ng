//! `StageCompleted` の永続化 DTO (**読む側**)。

use core_command_domain::orchestration::StageCompleted;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::intent_execution_event_dto::{slug_of, slug_spelling};

/// `StageCompleted` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageCompletedDto {
    stage: String,
}

impl StageCompletedDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(super) fn of(payload: &StageCompleted) -> StageCompletedDto {
        StageCompletedDto {
            stage: slug_spelling(payload.stage()),
        }
    }

    /// ドメインの材料へ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<StageCompleted, DtoDecodeError> {
        Ok(StageCompleted::new(slug_of(&self.stage, "stage")?))
    }
}
