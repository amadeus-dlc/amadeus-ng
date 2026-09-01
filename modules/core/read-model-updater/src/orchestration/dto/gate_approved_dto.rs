//! `GateApproved` の永続化 DTO (**読む側**)。

use core_command_domain::orchestration::GateApproved;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::intent_execution_event_dto::{slug_of, slug_spelling};

/// `GateApproved` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateApprovedDto {
    stage: String,
    user_input: Option<String>,
}

impl GateApprovedDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(super) fn of(payload: &GateApproved) -> GateApprovedDto {
        GateApprovedDto {
            stage: slug_spelling(payload.stage()),
            user_input: payload.user_input().map(str::to_string),
        }
    }

    /// ドメインの材料へ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<GateApproved, DtoDecodeError> {
        Ok(GateApproved::new(
            slug_of(&self.stage, "stage")?,
            self.user_input.clone(),
        ))
    }
}
