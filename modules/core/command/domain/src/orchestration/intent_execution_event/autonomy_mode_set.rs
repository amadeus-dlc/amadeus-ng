//! `AutonomyModeSet` — `IntentExecutionEvent::AutonomyModeSet` のペイロード。

use crate::orchestration::AutonomyMode;
use crate::orchestration::{IntentExecutionEventId, IntentExecutionId};

/// `AutonomyModeSet` のペイロード。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutonomyModeSet {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    mode: AutonomyMode,
}

impl AutonomyModeSet {
    /// 設定後のモード。
    #[must_use]
    pub const fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        mode: AutonomyMode,
    ) -> AutonomyModeSet {
        AutonomyModeSet {
            id,
            aggregate_id,
            mode,
        }
    }

    /// 設定後のモード。
    #[must_use]
    pub const fn mode(&self) -> AutonomyMode {
        self.mode
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
