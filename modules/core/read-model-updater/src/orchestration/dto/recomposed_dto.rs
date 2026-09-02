//! `Recomposed` の永続化 DTO (**読む側**)。

use core_command_domain::orchestration::Recomposed;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::intent_execution_event_dto::{aggregate_id_of, event_id_of, slug_spelling, slugs_of};

/// `Recomposed` の材料。**`id` (イベント自身の識別子) と `aggregate_id`
/// (どの集約の事実か) を先頭に置く並びが契約**である。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecomposedDto {
    id: String,
    aggregate_id: String,
    skipped: Vec<String>,
    added: Vec<String>,
}

impl RecomposedDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(super) fn of(payload: &Recomposed) -> RecomposedDto {
        RecomposedDto {
            id: payload.id().as_str().to_string(),
            aggregate_id: payload.aggregate_id().as_str().to_string(),
            skipped: payload.skipped().iter().map(slug_spelling).collect(),
            added: payload.added().iter().map(slug_spelling).collect(),
        }
    }

    /// ドメインの材料へ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<Recomposed, DtoDecodeError> {
        Ok(Recomposed::new(
            event_id_of(&self.id)?,
            aggregate_id_of(&self.aggregate_id)?,
            slugs_of(&self.skipped, "skipped")?,
            slugs_of(&self.added, "added")?,
        ))
    }
}
