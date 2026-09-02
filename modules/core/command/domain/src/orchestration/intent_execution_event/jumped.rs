//! `Jumped` — `IntentExecutionEvent::Jumped` のペイロード。

use crate::orchestration::{IntentExecutionEventId, IntentExecutionId};
use crate::workflow_definition::StageSlug;

/// `Jumped` のペイロード — 事実 (どこへ跳んだか) だけを運ぶ。
///
/// 方向・出発点・読み飛ばし列・巻き戻し列は載せない — すべて跳躍規則 (BR1.6) による導出で
/// あり、適用側 (集約) とリードモデル側 (RMU) がそれぞれ自分の状態 (カーソル・checkbox・
/// 実効プラン) から導く (オーナー裁定 2026-08-30「イベントに状態は含めるな」)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Jumped {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    target: StageSlug,
}

impl Jumped {
    /// 跳んだ先。
    #[must_use]
    pub const fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        target: StageSlug,
    ) -> Jumped {
        Jumped {
            id,
            aggregate_id,
            target,
        }
    }

    /// 跳んだ先。
    #[must_use]
    pub const fn target(&self) -> &StageSlug {
        &self.target
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
