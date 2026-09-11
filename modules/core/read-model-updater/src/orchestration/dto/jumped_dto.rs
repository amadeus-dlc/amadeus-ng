//! `Jumped` の永続化 DTO (**読む側**)。

use super::jump_observation_dto::JumpObservationDto;
use super::jump_scope_dto::JumpScopeDto;
use core_command_domain::orchestration::Jumped;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::intent_execution_event_dto::{aggregate_id_of, event_id_of, slug_of, slug_spelling};

/// `Jumped` の材料。**`id` (イベント自身の識別子) と `aggregate_id`
/// (どの集約の事実か) を先頭に置く並びが契約**である。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JumpedDto {
    id: String,
    aggregate_id: String,
    target: String,
    direction: String,
    observation: Option<JumpObservationDto>,
    /// 別 scope の実効計画 (直接 execute の `--scope`)。欄が無い行は「名指していない」。
    #[serde(default)]
    scope: Option<JumpScopeDto>,
}

impl JumpedDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(super) fn of(payload: &Jumped) -> JumpedDto {
        JumpedDto {
            id: payload.id().as_str().to_string(),
            aggregate_id: payload.aggregate_id().as_str().to_string(),
            target: slug_spelling(payload.target()),
            direction: direction_spelling(payload.direction()).into(),
            observation: payload.observation().map(JumpObservationDto::of),
            scope: payload.scope().map(JumpScopeDto::of),
        }
    }

    /// ドメインの材料へ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<Jumped, DtoDecodeError> {
        Ok(Jumped::new(
            event_id_of(&self.id)?,
            aggregate_id_of(&self.aggregate_id)?,
            slug_of(&self.target, "target")?,
            direction_of(&self.direction)?,
            self.observation
                .as_ref()
                .map(JumpObservationDto::to_domain)
                .transpose()?,
        )
        .with_scope(
            self.scope
                .as_ref()
                .map(JumpScopeDto::to_domain)
                .transpose()?,
        ))
    }
}

pub(super) fn direction_of(
    value: &str,
) -> Result<
    core_command_domain::orchestration::JumpDirection,
    super::dto_decode_error::DtoDecodeError,
> {
    use core_command_domain::orchestration::JumpDirection;
    match value {
        "forward" => Ok(JumpDirection::Forward),
        "backward" => Ok(JumpDirection::Backward),
        "redo" => Ok(JumpDirection::Redo),
        _ => Err(super::dto_decode_error::DtoDecodeError::malformed(
            "direction",
            value,
        )),
    }
}
const fn direction_spelling(
    direction: core_command_domain::orchestration::JumpDirection,
) -> &'static str {
    use core_command_domain::orchestration::JumpDirection;
    match direction {
        JumpDirection::Forward => "forward",
        JumpDirection::Backward => "backward",
        JumpDirection::Redo => "redo",
    }
}
