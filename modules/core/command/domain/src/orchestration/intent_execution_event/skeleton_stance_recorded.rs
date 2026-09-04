//! `SkeletonStanceRecorded` — `IntentExecutionEvent::SkeletonStanceRecorded` のペイロード。

use crate::orchestration::{IntentExecutionEventId, IntentExecutionId, SkeletonStance};

/// conductor が分類した walking-skeleton の stance を記録した事実。
///
/// 分類 (`## Walking Skeleton` の自由記述を 3 値へ落とす) はエンジンが計算できない唯一の
/// ゲート値であり、`report --skeleton-stance` の往復でここへ戻る。適用は
/// `skeleton_stance` を上書きする — 再記録は正当な操作である (upstream の
/// `setOrInsertField` と同じ意味論)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkeletonStanceRecorded {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    stance: SkeletonStance,
}

impl SkeletonStanceRecorded {
    /// 分類結果を束ねる。
    #[must_use]
    pub const fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        stance: SkeletonStance,
    ) -> SkeletonStanceRecorded {
        SkeletonStanceRecorded {
            id,
            aggregate_id,
            stance,
        }
    }

    /// 分類された stance。
    #[must_use]
    pub const fn stance(&self) -> SkeletonStance {
        self.stance
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
