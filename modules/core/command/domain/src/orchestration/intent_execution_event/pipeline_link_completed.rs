//! Pipeline linkの完了を受領した単一イベント。
use crate::orchestration::{IntentExecutionEventId, IntentExecutionId, PipelineReceipt};
/// PIPELINE_LINK_COMPLETEDを描く、受理済みの完了事実。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineLinkCompleted {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    receipt: PipelineReceipt,
}
impl PipelineLinkCompleted {
    /// 集約から返す事実または保存済み事実を組む。
    #[must_use]
    pub const fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        receipt: PipelineReceipt,
    ) -> Self {
        Self {
            id,
            aggregate_id,
            receipt,
        }
    }
    /// イベント自身の識別子。
    #[must_use]
    pub const fn id(&self) -> &IntentExecutionEventId {
        &self.id
    }
    /// 実行集約の識別子。
    #[must_use]
    pub const fn aggregate_id(&self) -> &IntentExecutionId {
        &self.aggregate_id
    }
    /// 受領した内容。
    #[must_use]
    pub const fn receipt(&self) -> &PipelineReceipt {
        &self.receipt
    }
}
