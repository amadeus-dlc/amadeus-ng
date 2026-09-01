//! `Jumped` の永続化 DTO (**読む側**)。

use core_command_domain::orchestration::Jumped;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::intent_execution_event_dto::{slug_of, slug_spelling};

/// `Jumped` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JumpedDto {
    target: String,
}

impl JumpedDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(super) fn of(payload: &Jumped) -> JumpedDto {
        JumpedDto {
            target: slug_spelling(payload.target()),
        }
    }

    /// ドメインの材料へ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<Jumped, DtoDecodeError> {
        Ok(Jumped::new(slug_of(&self.target, "target")?))
    }
}
