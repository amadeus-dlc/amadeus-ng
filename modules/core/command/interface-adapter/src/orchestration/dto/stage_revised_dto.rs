//! `StageRevisedDto` — `StageRevised` の材料。

use serde::{Deserialize, Serialize};

/// `StageRevised` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageRevisedDto {
    pub(super) stage: String,
}
