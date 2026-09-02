//! `StageSkipped` — `IntentExecutionEvent::StageSkipped` のペイロード。

use crate::orchestration::{IntentExecutionEventId, IntentExecutionId};
use crate::workflow_definition::StageSlug;

/// `StageSkipped` のペイロード。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageSkipped {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    stage: StageSlug,
    reason: String,
}

impl StageSkipped {
    /// 読み飛ばしたステージと、理由。次カーソルは載せない (導出 — オーナー裁定 2026-08-30)。
    #[must_use]
    pub const fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        stage: StageSlug,
        reason: String,
    ) -> StageSkipped {
        StageSkipped {
            id,
            aggregate_id,
            stage,
            reason,
        }
    }

    /// 読み飛ばしたステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }

    /// 読み飛ばしの理由 (逐語保持)。
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
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
