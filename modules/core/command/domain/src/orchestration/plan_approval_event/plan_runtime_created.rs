//! 共有承認集約に起きた事実。
use crate::orchestration::{PlanApprovalEventId, PlanApprovalRuntimeId};
/// イベント自身の識別子と、当時の内容を保持する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanRuntimeCreated {
    id: PlanApprovalEventId,
    aggregate_id: PlanApprovalRuntimeId,
}
impl PlanRuntimeCreated {
    /// 記録された事実を値から再構成する。
    #[must_use]
    pub const fn new(id: PlanApprovalEventId, aggregate_id: PlanApprovalRuntimeId) -> Self {
        Self { id, aggregate_id }
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
}
