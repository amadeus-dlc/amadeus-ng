//! `SkeletonStanceRecorded` の永続化 DTO (**読む側**)。

use core_command_domain::orchestration::SkeletonStanceRecorded;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::dto_vocabulary::{skeleton_stance_of, skeleton_stance_spelling};
use super::intent_execution_event_dto::{aggregate_id_of, event_id_of};

/// `SkeletonStanceRecorded` の材料。**`id` (イベント自身の識別子) と `aggregate_id`
/// (どの集約の事実か) を先頭に置く並びが契約**である。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkeletonStanceRecordedDto {
    id: String,
    aggregate_id: String,
    stance: String,
}

impl SkeletonStanceRecordedDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(super) fn of(payload: &SkeletonStanceRecorded) -> SkeletonStanceRecordedDto {
        SkeletonStanceRecordedDto {
            id: payload.id().as_str().to_string(),
            aggregate_id: payload.aggregate_id().as_str().to_string(),
            stance: skeleton_stance_spelling(payload.stance()).to_string(),
        }
    }

    /// ドメインの材料へ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<SkeletonStanceRecorded, DtoDecodeError> {
        Ok(SkeletonStanceRecorded::new(
            event_id_of(&self.id)?,
            aggregate_id_of(&self.aggregate_id)?,
            skeleton_stance_of(&self.stance, "stance")?,
        ))
    }
}
