//! 共有側で受領した計画回答を、元の実行へ監査記録した事実。
use crate::orchestration::{
    IntentExecutionEventId, IntentExecutionId, PlanAnswerInput, PlanApprovalOperationId,
};
/// 共有側で受領した計画回答を、元の実行へ監査記録した事実。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanAnswerLogged {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    operation_id: PlanApprovalOperationId,
    input: PlanAnswerInput,
}
impl PlanAnswerLogged {
    /// 検証済みの事実を構築する。
    #[must_use]
    pub const fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        operation_id: PlanApprovalOperationId,
        input: PlanAnswerInput,
    ) -> Self {
        Self {
            id,
            aggregate_id,
            operation_id,
            input,
        }
    }
    /// イベント自身の識別子。
    #[must_use]
    pub const fn id(&self) -> &IntentExecutionEventId {
        &self.id
    }
    /// 所属する実行。
    #[must_use]
    pub const fn aggregate_id(&self) -> &IntentExecutionId {
        &self.aggregate_id
    }
    /// 監査記録した回答操作。
    #[must_use]
    pub const fn operation_id(&self) -> &PlanApprovalOperationId {
        &self.operation_id
    }
    /// 受領時の文書・選択・観測先。
    #[must_use]
    pub const fn input(&self) -> &PlanAnswerInput {
        &self.input
    }
}
