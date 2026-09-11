//! 認証中のソース変更を検出し、受領を取り消した事実。
use crate::orchestration::{PlanApprovalEventId, PlanApprovalOperationId, PlanApprovalRuntimeId};
/// 認証中のソース変更を検出し、受領を取り消した事実。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanAnswerAborted {
    id: PlanApprovalEventId,
    aggregate_id: PlanApprovalRuntimeId,
    operation_id: PlanApprovalOperationId,
}
impl PlanAnswerAborted {
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
