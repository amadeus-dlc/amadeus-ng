//! `StageRevised` の永続化 DTO (**読む側**)。

use core_command_domain::orchestration::StageRevised;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::intent_execution_event_dto::{slug_of, slug_spelling};

/// `StageRevised` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageRevisedDto {
    stage: String,
}

impl StageRevisedDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(super) fn of(payload: &StageRevised) -> StageRevisedDto {
        StageRevisedDto {
            stage: slug_spelling(payload.stage()),
        }
    }

    /// ドメインの材料へ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<StageRevised, DtoDecodeError> {
        Ok(StageRevised::new(slug_of(&self.stage, "stage")?))
    }
}
