//! `JumpedDto` — `Jumped` の材料。

use serde::{Deserialize, Serialize};

/// `Jumped` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JumpedDto {
    pub(super) target: String,
}
