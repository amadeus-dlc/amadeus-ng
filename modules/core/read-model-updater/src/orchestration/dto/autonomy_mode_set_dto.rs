//! `AutonomyModeSet` の永続化 DTO (**読む側**)。

use core_command_domain::orchestration::AutonomyModeSet;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::dto_vocabulary::{autonomy_of, autonomy_spelling};
use super::intent_execution_event_dto::{aggregate_id_of, event_id_of};

/// `AutonomyModeSet` の材料。**`id` (イベント自身の識別子) と `aggregate_id`
/// (どの集約の事実か) を先頭に置く並びが契約**である。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomyModeSetDto {
    id: String,
    aggregate_id: String,
    mode: String,
}

impl AutonomyModeSetDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(super) fn of(payload: &AutonomyModeSet) -> AutonomyModeSetDto {
        AutonomyModeSetDto {
            id: payload.id().as_str().to_string(),
            aggregate_id: payload.aggregate_id().as_str().to_string(),
            mode: autonomy_spelling(payload.mode()).to_string(),
        }
    }

    /// ドメインの材料へ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<AutonomyModeSet, DtoDecodeError> {
        Ok(AutonomyModeSet::new(
            event_id_of(&self.id)?,
            aggregate_id_of(&self.aggregate_id)?,
            autonomy_of(&self.mode)?,
        ))
    }
}
