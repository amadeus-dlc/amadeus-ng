//! `ReviewCompletedDto` — `ReviewCompleted` の材料。

use serde::{Deserialize, Serialize};

/// `ReviewCompleted` の材料。**`id` (イベント自身の識別子) と `aggregate_id`
/// (どの集約の事実か) を先頭に置く並びが契約**である。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewCompletedDto {
    pub(super) id: String,
    pub(super) aggregate_id: String,
    pub(super) stage: String,
    pub(super) reviewer: String,
    pub(super) iteration: u32,
    pub(super) verdict: String,
}
