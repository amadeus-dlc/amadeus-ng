//! 通常指示の発行と共有承認の失効を対応させる操作。
use super::{IntentExecutionId, PlanApprovalOperationId};
use crate::workspace::SpaceName;
/// 失効の準備。元の指示発行先を保持して、再開時に完了の有無を確認する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanInvalidation {
    id: PlanApprovalOperationId,
    space: SpaceName,
    execution_id: IntentExecutionId,
}
impl PlanInvalidation {
    /// 発行を始める前に対象を固定する。
    #[must_use]
    pub const fn new(
        id: PlanApprovalOperationId,
        space: SpaceName,
        execution_id: IntentExecutionId,
    ) -> Self {
        Self {
            id,
            space,
            execution_id,
        }
    }
    /// 複数更新を同じ操作として対応させる識別子。
    #[must_use]
    pub const fn id(&self) -> &PlanApprovalOperationId {
        &self.id
    }
    /// 指示が属するspace。
    #[must_use]
    pub const fn space(&self) -> &SpaceName {
        &self.space
    }
    /// 指示が属する実行。
    #[must_use]
    pub const fn execution_id(&self) -> &IntentExecutionId {
        &self.execution_id
    }
}
