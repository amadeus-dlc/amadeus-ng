//! `StartedDto` — `Started` の材料。

use serde::{Deserialize, Serialize};

use super::intent_dto::StageEntryDto;

/// `Started` の材料 — genesis の 3 点 (実行 id・intent id・解決済み計画)。
///
/// **フィールド名と並びが契約**である。計画の写しを運ぶのは、実行の歴史が自ストリーム
/// だけで再生できるための条件である (`coding-rules/aggregate-commands.md`)。1 要素の綴りは
/// [`IntentDto`] の `StageEntryDto` を共有するので、intent 面と同じバイトになる。
///
/// [`IntentDto`]: super::intent_dto::IntentDto
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartedDto {
    pub(super) id: String,
    pub(super) aggregate_id: String,
    pub(super) intent_id: String,
    pub(super) stages: Vec<StageEntryDto>,
}
