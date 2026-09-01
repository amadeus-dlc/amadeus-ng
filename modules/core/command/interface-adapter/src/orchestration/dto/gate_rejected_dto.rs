//! `GateRejectedDto` — `GateRejected` の材料。

use serde::{Deserialize, Serialize};

/// `GateRejected` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateRejectedDto {
    pub(super) stage: String,
    pub(super) feedback: Option<String>,
}
