//! TaskUpdateが指す作業stageへ状態を同期した事実。
use crate::orchestration::{IntentExecutionEventId, IntentExecutionId};
use crate::workflow_definition::StageSlug;
/// stageの同期はゲート承認やstage開始監査を意味しない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSynchronized {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    stage: StageSlug,
}
impl TaskSynchronized {
    /// 記録先と同期したstageを束ねる。
    #[must_use]
    pub const fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        stage: StageSlug,
    ) -> Self {
        Self {
            id,
            aggregate_id,
            stage,
        }
    }
    /// 同期事実の識別子。
    #[must_use]
    pub const fn id(&self) -> &IntentExecutionEventId {
        &self.id
    }
    /// 同期した実行。
    #[must_use]
    pub const fn aggregate_id(&self) -> &IntentExecutionId {
        &self.aggregate_id
    }
    /// 同期したstage。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }
}
