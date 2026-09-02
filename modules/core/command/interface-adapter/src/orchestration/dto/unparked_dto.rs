//! `UnparkedDto` — `Unparked` の材料。

use serde::{Deserialize, Serialize};

/// `Unparked` の材料 — ドメインの材料は無いが、識別子は運ぶ。
///
/// かつては単位変種 (`"Unparked"` という裸の文字列) だった。ドメインイベントは
/// エンティティの一種であり、材料の有無にかかわらず `id` と `aggregate_id` を持つので、
/// 行の形も構造体になった (オーナー裁定 2026-09-02)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnparkedDto {
    pub(super) id: String,
    pub(super) aggregate_id: String,
}
