//! ワークスペース全体の承認操作が観測された実行の所在。
use super::IntentExecutionId;
use crate::workspace::SpaceName;
/// 他の集約は所在地とIDで参照し、状態を埋め込まない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanApprovalOrigin {
    space: SpaceName,
    execution_id: IntentExecutionId,
}
impl PlanApprovalOrigin {
    /// 観測先を固定する。
    #[must_use]
    pub const fn new(space: SpaceName, execution_id: IntentExecutionId) -> Self {
        Self {
            space,
            execution_id,
        }
    }
    /// 観測先のspace。
    #[must_use]
    pub const fn space(&self) -> &SpaceName {
        &self.space
    }
    /// 観測先の実行。
    #[must_use]
    pub const fn execution_id(&self) -> &IntentExecutionId {
        &self.execution_id
    }
}
