//! `Recomposed` — `IntentExecutionEvent::Recomposed` のペイロード。

use crate::orchestration::{IntentExecutionEventId, IntentExecutionId, StageSlugSet};

/// `Recomposed` のペイロード — 事実 (どの反転が起きたか) だけを運ぶ。
///
/// 適用後の in-scope 列は載せない — 適用後の状態であり、適用側とリードモデル側が自分の
/// 実効プランから導く (オーナー裁定 2026-08-30)。1 コマンドの複数反転は 1 イベント (C5)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recomposed {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    skipped: StageSlugSet,
    added: StageSlugSet,
}

impl Recomposed {
    /// EXECUTE → SKIP にした列、SKIP → EXECUTE にした列。
    #[must_use]
    pub const fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        skipped: StageSlugSet,
        added: StageSlugSet,
    ) -> Recomposed {
        Recomposed {
            id,
            aggregate_id,
            skipped,
            added,
        }
    }

    /// EXECUTE → SKIP に反転したステージの集合 (辞書順)。
    ///
    /// 反転は位置ごとに独立なので集合で足りる。**文書順で描く投影は並べ直す**（計画の位置で
    /// 引き直す）— 型側の順序は監査行の都合で変えない。
    #[must_use]
    pub const fn skipped(&self) -> &StageSlugSet {
        &self.skipped
    }

    /// SKIP → EXECUTE に反転したステージの集合 (辞書順)。
    #[must_use]
    pub const fn added(&self) -> &StageSlugSet {
        &self.added
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
