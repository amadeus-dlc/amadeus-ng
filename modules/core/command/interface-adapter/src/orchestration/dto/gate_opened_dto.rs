//! `GateOpenedDto` — `GateOpened` の材料。

use serde::{Deserialize, Serialize};

/// `GateOpened` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateOpenedDto {
    pub(super) stage: String,
    pub(super) artifacts: Vec<String>,
}
