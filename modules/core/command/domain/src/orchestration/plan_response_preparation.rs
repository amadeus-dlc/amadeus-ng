//! 観測時の発行回と意味を固定した、人間応答の保存準備。
use super::{PlanApprovalOperationId, PlanApprovalOrigin, PlanChoice, PlanSession};
/// 別の質問が後から提示されても、応答をその質問で解釈し直さない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanResponsePreparation {
    id: PlanApprovalOperationId,
    origin: PlanApprovalOrigin,
    occurrence_id: PlanApprovalOperationId,
    session: PlanSession,
    response: String,
    choice: Option<PlanChoice>,
}
impl PlanResponsePreparation {
    /// 観測時に確定した材料を束ねる。
    #[must_use]
    pub const fn new(
        id: PlanApprovalOperationId,
        origin: PlanApprovalOrigin,
        occurrence_id: PlanApprovalOperationId,
        session: PlanSession,
        response: String,
        choice: Option<PlanChoice>,
    ) -> Self {
        Self {
            id,
            origin,
            occurrence_id,
            session,
            response,
            choice,
        }
    }
    /// 保存・配信を対応付ける操作ID。
    #[must_use]
    pub const fn id(&self) -> &PlanApprovalOperationId {
        &self.id
    }
    /// 観測された実行。
    #[must_use]
    pub const fn origin(&self) -> &PlanApprovalOrigin {
        &self.origin
    }
    /// 観測時に提示されていた発行回。
    #[must_use]
    pub const fn occurrence_id(&self) -> &PlanApprovalOperationId {
        &self.occurrence_id
    }
    /// 実際のセッション。
    #[must_use]
    pub const fn session(&self) -> &PlanSession {
        &self.session
    }
    /// 観測した応答原文。
    #[must_use]
    pub fn response(&self) -> &str {
        &self.response
    }
    /// 当時の提示へ照合した意味。無関係な発言はNone。
    #[must_use]
    pub const fn choice(&self) -> Option<PlanChoice> {
        self.choice
    }
}
