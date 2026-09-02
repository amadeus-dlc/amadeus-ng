//! `Unparked` — `IntentExecutionEvent::Unparked` のペイロード。

use crate::orchestration::{IntentExecutionEventId, IntentExecutionId};

/// `Unparked` のペイロード — park マーカーの除去。
///
/// 位置の材料は運ばない (`parked_at` から復元される)。それでも**単位変種ではなく構造体**
/// なのは、ドメインイベントがエンティティの一種であり、材料の有無にかかわらず自前の
/// 識別子と「どの集約の事実か」を持つからである (オーナー裁定 2026-09-02、
/// `coding-rules/domain-object-kinds.md`)。C5 の `payload: {}` は**ドメインの材料が
/// 空である**という意味であって、イベントが同一性を持たないという意味ではない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unparked {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
}

impl Unparked {
    /// イベントの識別子と、事実が起きた実行の識別子を束ねる。
    #[must_use]
    pub const fn new(id: IntentExecutionEventId, aggregate_id: IntentExecutionId) -> Unparked {
        Unparked { id, aggregate_id }
    }

    /// このイベント自身の識別子 — ドメインイベントはエンティティの一種なので自前の id を
    /// 持つ (`coding-rules/domain-object-kinds.md`)。
    #[must_use]
    pub const fn id(&self) -> &IntentExecutionEventId {
        &self.id
    }

    /// **どの集約の事実か** — この事実が起きた実行の識別子。
    #[must_use]
    pub const fn aggregate_id(&self) -> &IntentExecutionId {
        &self.aggregate_id
    }
}
