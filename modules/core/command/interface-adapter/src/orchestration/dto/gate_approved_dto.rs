//! `GateApprovedDto` — `GateApproved` の材料。

use serde::{Deserialize, Serialize};

/// `GateApproved` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateApprovedDto {
    pub(super) stage: String,
    pub(super) user_input: Option<String>,
}
