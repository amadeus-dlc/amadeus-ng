//! `Started` の永続化 DTO (**読む側**)。

use core_command_domain::orchestration::{IntentId, Started};
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;

/// `Started` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartedDto {
    intent_id: String,
}

impl StartedDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(super) fn of(payload: &Started) -> StartedDto {
        StartedDto {
            intent_id: payload.intent_id().as_str().to_string(),
        }
    }

    /// ドメインの材料へ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<Started, DtoDecodeError> {
        Ok(Started::new(IntentId::parse(&self.intent_id).map_err(
            |_| DtoDecodeError::malformed("intent_id", &self.intent_id),
        )?))
    }
}
