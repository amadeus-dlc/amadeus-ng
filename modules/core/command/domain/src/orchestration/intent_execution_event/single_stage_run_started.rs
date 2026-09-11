//! 単独pipeline試行の開始事実。
use crate::orchestration::{IntentExecutionEventId, IntentExecutionId};
use crate::workflow_definition::StageSlug;
/// 単独実行のSTAGE_STARTEDを一度だけ描く。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SingleStageRunStarted {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    stage: StageSlug,
}
impl SingleStageRunStarted {
    /// 保存済みまたは集約が生成する開始事実を組む。
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
    /// イベントID。
    #[must_use]
    pub const fn id(&self) -> &IntentExecutionEventId {
        &self.id
    }
    /// 実行ID。
    #[must_use]
    pub const fn aggregate_id(&self) -> &IntentExecutionId {
        &self.aggregate_id
    }
    /// 開始するstage。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }
}
