//! 共有承認の失効を伴う指示発行を始めた事実。
use crate::orchestration::{PlanApprovalEventId, PlanApprovalRuntimeId, PlanInvalidation};
/// 発行先と同じ操作識別子を保持する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanInvalidationPrepared {
    id: PlanApprovalEventId,
    aggregate_id: PlanApprovalRuntimeId,
    invalidation: PlanInvalidation,
}
impl PlanInvalidationPrepared {
    /// 準備した事実を束ねる。
    #[must_use]
    pub const fn new(
        id: PlanApprovalEventId,
        aggregate_id: PlanApprovalRuntimeId,
        invalidation: PlanInvalidation,
    ) -> Self {
        Self {
            id,
            aggregate_id,
            invalidation,
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
    /// 記録されたinvalidation。
    #[must_use]
    pub const fn invalidation(&self) -> &PlanInvalidation {
        &self.invalidation
    }
}
