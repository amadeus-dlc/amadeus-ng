//! 元の実行への監査保存を確認し、計画回答を完了した事実。
use crate::orchestration::{PlanApprovalEventId, PlanApprovalOperationId, PlanApprovalRuntimeId};
/// 元の実行への監査保存を確認し、計画回答を完了した事実。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanAnswerCompleted {
    id: PlanApprovalEventId,
    aggregate_id: PlanApprovalRuntimeId,
    operation_id: PlanApprovalOperationId,
}
impl PlanAnswerCompleted {
    /// 検証済みの事実を構築する。
    #[must_use]
    pub const fn new(
        id: PlanApprovalEventId,
        aggregate_id: PlanApprovalRuntimeId,
        operation_id: PlanApprovalOperationId,
    ) -> Self {
        Self {
            id,
            aggregate_id,
            operation_id,
        }
    }
    /// イベント自身の識別子。
    #[must_use]
    pub const fn id(&self) -> &PlanApprovalEventId {
        &self.id
    }
    /// 所属する共有集約。
    #[must_use]
    pub const fn aggregate_id(&self) -> &PlanApprovalRuntimeId {
        &self.aggregate_id
    }
    /// 完了した回答操作。
    #[must_use]
    pub const fn operation_id(&self) -> &PlanApprovalOperationId {
        &self.operation_id
    }
}
