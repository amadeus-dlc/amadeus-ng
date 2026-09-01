//! `GateOpened` の永続化 DTO (**読む側**)。

use core_command_domain::orchestration::GateOpened;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::intent_execution_event_dto::{slug_of, slug_spelling};

/// `GateOpened` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateOpenedDto {
    stage: String,
    artifacts: Vec<String>,
}

impl GateOpenedDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(super) fn of(payload: &GateOpened) -> GateOpenedDto {
        GateOpenedDto {
            stage: slug_spelling(payload.stage()),
            artifacts: payload.artifacts().to_vec(),
        }
    }

    /// ドメインの材料へ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<GateOpened, DtoDecodeError> {
        Ok(GateOpened::new(
            slug_of(&self.stage, "stage")?,
            self.artifacts.clone(),
        ))
    }
}
