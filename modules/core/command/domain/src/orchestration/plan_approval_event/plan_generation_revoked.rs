//! 公開後のソースを照合して実装開始の失効を記録した事実。
use crate::orchestration::{PlanApprovalEventId, PlanApprovalOperationId, PlanApprovalRuntimeId};
/// 公開後のソースを照合して実装開始の失効を記録した事実。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanGenerationRevoked {
    id: PlanApprovalEventId,
    aggregate_id: PlanApprovalRuntimeId,
    operation_id: PlanApprovalOperationId,
}
impl PlanGenerationRevoked {
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
    /// 照合した開始操作。
    #[must_use]
    pub const fn operation_id(&self) -> &PlanApprovalOperationId {
        &self.operation_id
    }
}
