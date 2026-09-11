//! 人間応答の観測先・発行回・原文を固定した事実。
use crate::orchestration::{PlanApprovalEventId, PlanApprovalRuntimeId, PlanResponsePreparation};
/// 他ストアへ反映する前に耐久保存する材料。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanResponsePrepared {
    id: PlanApprovalEventId,
    aggregate_id: PlanApprovalRuntimeId,
    preparation: PlanResponsePreparation,
}
impl PlanResponsePrepared {
    /// 記録された準備を束ねる。
    #[must_use]
    pub const fn new(
        id: PlanApprovalEventId,
        aggregate_id: PlanApprovalRuntimeId,
        preparation: PlanResponsePreparation,
    ) -> Self {
        Self {
            id,
            aggregate_id,
            preparation,
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
    /// 記録されたpreparation。
    #[must_use]
    pub const fn preparation(&self) -> &PlanResponsePreparation {
        &self.preparation
    }
}
