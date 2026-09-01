//! `AutonomyModeSet` の永続化 DTO (**読む側**)。

use core_command_domain::orchestration::AutonomyModeSet;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::dto_vocabulary::{autonomy_of, autonomy_spelling};

/// `AutonomyModeSet` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomyModeSetDto {
    mode: String,
}

impl AutonomyModeSetDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(super) fn of(payload: &AutonomyModeSet) -> AutonomyModeSetDto {
        AutonomyModeSetDto {
            mode: autonomy_spelling(payload.mode()).to_string(),
        }
    }

    /// ドメインの材料へ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<AutonomyModeSet, DtoDecodeError> {
        Ok(AutonomyModeSet::new(autonomy_of(&self.mode)?))
    }
}
