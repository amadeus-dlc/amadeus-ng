//! ハーネスから届いた応答の観測。無人運転は人間の権限を与えない。
use crate::orchestration::{IntentExecutionEventId, IntentExecutionId};
/// フック経由で受け取った応答の事実。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptObserved {
    approval_observation_id: Option<crate::orchestration::PlanApprovalOperationId>,
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    session: String,
    response: String,
    unattended: bool,
}
impl PromptObserved {
    /// 共有側が先に固定した観測操作を対応付ける。
    #[must_use]
    pub fn with_approval_observation(
        mut self,
        id: Option<crate::orchestration::PlanApprovalOperationId>,
    ) -> Self {
        self.approval_observation_id = id;
        self
    }
    /// 保護された応答の観測操作。
    #[must_use]
    pub const fn approval_observation_id(
        &self,
    ) -> Option<&crate::orchestration::PlanApprovalOperationId> {
        self.approval_observation_id.as_ref()
    }

    /// 識別子とフックが観測した入力から組む。
    #[must_use]
    pub fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        session: impl Into<String>,
        response: impl Into<String>,
        unattended: bool,
    ) -> Self {
        Self {
            approval_observation_id: None,
            id,
            aggregate_id,
            session: session.into(),
            response: response.into(),
            unattended,
        }
    }
    /// イベント識別子。
    #[must_use]
    pub const fn id(&self) -> &IntentExecutionEventId {
        &self.id
    }
    /// 対象実行。
    #[must_use]
    pub const fn aggregate_id(&self) -> &IntentExecutionId {
        &self.aggregate_id
    }
    /// ハーネスのセッション識別子。旧封筒は空。
    #[must_use]
    pub fn session(&self) -> &str {
        &self.session
    }
    /// 番号を含めた応答の元の文字列。
    #[must_use]
    pub fn response(&self) -> &str {
        &self.response
    }
    /// 無人運転の入力か。
    #[must_use]
    pub const fn unattended(&self) -> bool {
        self.unattended
    }
}
