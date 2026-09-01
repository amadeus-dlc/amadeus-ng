//! `GateRejected` の永続化 DTO (**読む側**)。

use core_command_domain::orchestration::GateRejected;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::intent_execution_event_dto::{slug_of, slug_spelling};

/// `GateRejected` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateRejectedDto {
    stage: String,
    feedback: Option<String>,
}

impl GateRejectedDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(super) fn of(payload: &GateRejected) -> GateRejectedDto {
        GateRejectedDto {
            stage: slug_spelling(payload.stage()),
            feedback: payload.feedback().map(str::to_string),
        }
    }

    /// ドメインの材料へ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<GateRejected, DtoDecodeError> {
        Ok(GateRejected::new(
            slug_of(&self.stage, "stage")?,
            self.feedback.clone(),
        ))
    }
}
