//! `JumpedDto` — `Jumped` の材料。

use super::jump_observation_dto::JumpObservationDto;
use super::jump_scope_dto::JumpScopeDto;
use serde::{Deserialize, Serialize};

/// `Jumped` の材料。**`id` (イベント自身の識別子) と `aggregate_id`
/// (どの集約の事実か) を先頭に置く並びが契約**である。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JumpedDto {
    pub(super) id: String,
    pub(super) aggregate_id: String,
    pub(super) target: String,
    direction: String,
    observation: Option<JumpObservationDto>,
    /// 別 scope の実効計画 (直接 execute の `--scope`)。欄が無い行は「名指していない」。
    #[serde(default)]
    scope: Option<JumpScopeDto>,
}

impl JumpedDto {
    pub(super) fn scope(
        &self,
    ) -> Result<
        Option<core_command_domain::orchestration::JumpScope>,
        super::dto_decode_error::DtoDecodeError,
    > {
        self.scope.as_ref().map(JumpScopeDto::to_domain).transpose()
    }
    pub(super) fn direction(
        &self,
    ) -> Result<
        core_command_domain::orchestration::JumpDirection,
        super::dto_decode_error::DtoDecodeError,
    > {
        direction_of(&self.direction)
    }
    pub(super) fn of(payload: &core_command_domain::orchestration::Jumped) -> Self {
        Self {
            id: payload.id().as_str().into(),
            aggregate_id: payload.aggregate_id().as_str().into(),
            target: payload.target().as_str().into(),
            direction: direction_spelling(payload.direction()).into(),
            observation: payload.observation().map(JumpObservationDto::of),
            scope: payload.scope().map(JumpScopeDto::of),
        }
    }
    pub(super) fn observation(
        &self,
    ) -> Result<
        Option<core_command_domain::orchestration::JumpObservation>,
        super::dto_decode_error::DtoDecodeError,
    > {
        self.observation
            .as_ref()
            .map(JumpObservationDto::to_domain)
            .transpose()
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
