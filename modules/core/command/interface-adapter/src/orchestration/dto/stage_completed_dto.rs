//! `StageCompletedDto` — `StageCompleted` の材料。

use serde::{Deserialize, Serialize};

/// `StageCompleted` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageCompletedDto {
    pub(super) stage: String,
}
