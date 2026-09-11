//! 計画承認の質問コマンドへの入力。
use core_command_domain::orchestration::{
    DecisionPrompt, PlanApprovalInput, PlanApprovalOperationId, PlanSession,
};
/// 呼出側が識別する操作と、提示する文書・質問・2択を束ねる。
#[derive(Debug, Clone)]
pub struct PlanDecisionRequest {
    operation_id: PlanApprovalOperationId,
    input: PlanApprovalInput,
    prompt: DecisionPrompt,
    session: PlanSession,
    exact: bool,
}
impl PlanDecisionRequest {
    /// 入力境界で解決した対象と原文を保持する。
    #[must_use]
    pub const fn new(
        operation_id: PlanApprovalOperationId,
        input: PlanApprovalInput,
        prompt: DecisionPrompt,
        session: PlanSession,
        exact: bool,
    ) -> Self {
        Self {
            operation_id,
            input,
            prompt,
            session,
            exact,
        }
    }
    /// 操作の識別子。
    #[must_use]
    pub const fn operation_id(&self) -> &PlanApprovalOperationId {
        &self.operation_id
    }
    /// 読み取った現在の文書と規則。
    #[must_use]
    pub const fn input(&self) -> &PlanApprovalInput {
        &self.input
    }
    /// 質問の原文。
    #[must_use]
    pub const fn prompt(&self) -> &DecisionPrompt {
        &self.prompt
    }
    /// 提示するセッション。
    #[must_use]
    pub const fn session(&self) -> &PlanSession {
        &self.session
    }
    /// ラベルの完全一致を要求するか。
    #[must_use]
    pub const fn exact(&self) -> bool {
        self.exact
    }
}
