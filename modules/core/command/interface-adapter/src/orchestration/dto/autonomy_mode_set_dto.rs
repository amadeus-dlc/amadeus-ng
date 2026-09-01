//! `AutonomyModeSetDto` — `AutonomyModeSet` の材料。

use serde::{Deserialize, Serialize};

/// `AutonomyModeSet` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomyModeSetDto {
    pub(super) mode: String,
}
