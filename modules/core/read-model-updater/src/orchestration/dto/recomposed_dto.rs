//! `Recomposed` の永続化 DTO (**読む側**)。

use core_command_domain::orchestration::Recomposed;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::intent_execution_event_dto::{slug_spelling, slugs_of};

/// `Recomposed` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecomposedDto {
    skipped: Vec<String>,
    added: Vec<String>,
}

impl RecomposedDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(super) fn of(payload: &Recomposed) -> RecomposedDto {
        RecomposedDto {
            skipped: payload.skipped().iter().map(slug_spelling).collect(),
            added: payload.added().iter().map(slug_spelling).collect(),
        }
    }

    /// ドメインの材料へ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<Recomposed, DtoDecodeError> {
        Ok(Recomposed::new(
            slugs_of(&self.skipped, "skipped")?,
            slugs_of(&self.added, "added")?,
        ))
    }
}
