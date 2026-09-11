//! 元の指示発行の成否を確認して、共有承認の保留を解いた事実。
use crate::orchestration::{PlanApprovalEventId, PlanApprovalOperationId, PlanApprovalRuntimeId};
/// 発行済みの場合だけ、全spaceの承認を失効させる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanInvalidationResolved {
    id: PlanApprovalEventId,
    aggregate_id: PlanApprovalRuntimeId,
    operation_id: PlanApprovalOperationId,
    published: bool,
}
impl PlanInvalidationResolved {
    /// 確認した発行結果を束ねる。
    #[must_use]
    pub const fn new(
        id: PlanApprovalEventId,
        aggregate_id: PlanApprovalRuntimeId,
        operation_id: PlanApprovalOperationId,
        published: bool,
    ) -> Self {
        Self {
            id,
            aggregate_id,
            operation_id,
            published,
        }
    }
    /// 記録されたid。
    #[must_use]
    pub const fn id(&self) -> &PlanApprovalEventId {
        &self.id
    }
    /// 記録されたaggregate_id。
    #[must_use]
    pub const fn aggregate_id(&self) -> &PlanApprovalRuntimeId {
        &self.aggregate_id
    }
    /// 記録されたoperation_id。
    #[must_use]
    pub const fn operation_id(&self) -> &PlanApprovalOperationId {
        &self.operation_id
    }
    /// 記録されたpublished。
    #[must_use]
    pub const fn published(&self) -> &bool {
        &self.published
    }
}
