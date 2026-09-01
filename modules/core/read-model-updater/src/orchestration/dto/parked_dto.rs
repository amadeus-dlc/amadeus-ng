//! `Parked` の永続化 DTO (**読む側**)。

use core_command_domain::orchestration::Parked;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::intent_execution_event_dto::{slug_of, slug_spelling};

/// `Parked` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParkedDto {
    stage: String,
}

impl ParkedDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(super) fn of(payload: &Parked) -> ParkedDto {
        ParkedDto {
            stage: slug_spelling(payload.stage()),
        }
    }

    /// ドメインの材料へ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<Parked, DtoDecodeError> {
        Ok(Parked::new(slug_of(&self.stage, "stage")?))
    }
}
