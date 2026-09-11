//! 計画回答を受領するコマンドへの入力。
use core_command_domain::orchestration::{
    PlanApprovalInput, PlanApprovalOperationId, PlanApprovalOrigin, PlanChoice, PlanSession,
};
/// 表示値を返さず、呼出側の操作IDで結果を識別する。
#[derive(Debug, Clone)]
pub struct PlanAnswerRequest {
    operation_id: PlanApprovalOperationId,
    input: PlanApprovalInput,
    origin: PlanApprovalOrigin,
    stage: String,
    session: PlanSession,
    choice: PlanChoice,
}
impl PlanAnswerRequest {
    /// 入力境界で得た値を束ねる。
    #[must_use]
    pub const fn new(
        operation_id: PlanApprovalOperationId,
        input: PlanApprovalInput,
        origin: PlanApprovalOrigin,
        stage: String,
        session: PlanSession,
        choice: PlanChoice,
    ) -> Self {
        Self {
            operation_id,
            input,
            origin,
            stage,
            session,
            choice,
        }
    }
    /// 回答操作の識別子。
    #[must_use]
    pub const fn operation_id(&self) -> &PlanApprovalOperationId {
        &self.operation_id
    }
    /// 現在の文書と参照情報。
    #[must_use]
    pub const fn input(&self) -> &PlanApprovalInput {
        &self.input
    }
    /// 監査記録の所有者。
    #[must_use]
    pub const fn origin(&self) -> &PlanApprovalOrigin {
        &self.origin
    }
    /// 対象ステージ。
    #[must_use]
    pub fn stage(&self) -> &str {
        &self.stage
    }
    /// 呼出セッション。
    #[must_use]
    pub const fn session(&self) -> &PlanSession {
        &self.session
    }
    /// 受領する選択。
    #[must_use]
    pub const fn choice(&self) -> &PlanChoice {
        &self.choice
    }
}
