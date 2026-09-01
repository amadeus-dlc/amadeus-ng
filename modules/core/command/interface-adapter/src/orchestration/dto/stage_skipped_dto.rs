//! `StageSkippedDto` — `StageSkipped` の材料。

use serde::{Deserialize, Serialize};

/// `StageSkipped` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageSkippedDto {
    pub(super) stage: String,
    pub(super) reason: String,
}
