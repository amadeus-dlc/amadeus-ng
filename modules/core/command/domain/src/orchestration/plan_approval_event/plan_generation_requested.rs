//! 文書・受領・直前ソースを照合して実装開始を要求した事実。
use crate::orchestration::{PlanApprovalEventId, PlanApprovalRuntimeId, PlanGeneration};
/// 開始許可の公開と確定を同じ操作IDで追跡する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanGenerationRequested {
    id: PlanApprovalEventId,
    aggregate_id: PlanApprovalRuntimeId,
    generation: PlanGeneration,
}
impl PlanGenerationRequested {
    /// 受領の事実を束ねる。
    #[must_use]
    pub const fn new(
        id: PlanApprovalEventId,
        aggregate_id: PlanApprovalRuntimeId,
        generation: PlanGeneration,
    ) -> Self {
        Self {
            id,
            aggregate_id,
            generation,
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
    /// 記録されたgeneration。
    #[must_use]
    pub const fn generation(&self) -> &PlanGeneration {
        &self.generation
    }
}
