//! `ParkedDto` — `Parked` の材料。

use serde::{Deserialize, Serialize};

/// `Parked` の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParkedDto {
    pub(super) stage: String,
}
