//! `StartedDto` — `Started` の材料。

use serde::{Deserialize, Serialize};

/// `Started` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartedDto {
    pub(super) intent_id: String,
}
