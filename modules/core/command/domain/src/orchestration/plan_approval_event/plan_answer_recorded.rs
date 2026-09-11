//! 提示・応答・文書を照合して計画回答を受領した事実。
use crate::orchestration::{PlanAnswer, PlanApprovalEventId, PlanApprovalRuntimeId};
/// 監査配送の完了前も、同じ操作IDで受領を識別する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanAnswerRecorded {
    id: PlanApprovalEventId,
    aggregate_id: PlanApprovalRuntimeId,
    answer: PlanAnswer,
}
impl PlanAnswerRecorded {
    /// 受領の事実を束ねる。
    #[must_use]
    pub const fn new(
        id: PlanApprovalEventId,
        aggregate_id: PlanApprovalRuntimeId,
        answer: PlanAnswer,
    ) -> Self {
        Self {
            id,
            aggregate_id,
            answer,
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
    /// 記録されたanswer。
    #[must_use]
    pub const fn answer(&self) -> &PlanAnswer {
        &self.answer
    }
}
