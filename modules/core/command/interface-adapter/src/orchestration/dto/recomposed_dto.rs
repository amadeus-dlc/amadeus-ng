//! `RecomposedDto` — `Recomposed` の材料。

use serde::{Deserialize, Serialize};

/// `Recomposed` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecomposedDto {
    pub(super) skipped: Vec<String>,
    pub(super) added: Vec<String>,
}
