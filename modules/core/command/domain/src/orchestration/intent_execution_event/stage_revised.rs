//! `StageRevised` — `IntentExecutionEvent::StageRevised` のペイロード。

use crate::orchestration::{IntentExecutionEventId, IntentExecutionId};
use crate::workflow_definition::StageSlug;

/// `StageRevised` のペイロード。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageRevised {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    stage: StageSlug,
}

impl StageRevised {
    /// ゲートに再入したステージ。
    #[must_use]
    pub const fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        stage: StageSlug,
    ) -> StageRevised {
        StageRevised {
            id,
            aggregate_id,
            stage,
        }
    }

    /// ゲートに再入したステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
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
