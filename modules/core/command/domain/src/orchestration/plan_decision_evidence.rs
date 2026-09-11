//! 計画承認の質問として監査に残す、対象と呼出セッションの根拠。
use super::{PlanApprovalEvidence, PlanSession};
/// 提示内容の検証結果。人間の回答や承認受領を意味しない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanDecisionEvidence {
    evidence: PlanApprovalEvidence,
    session: PlanSession,
}
impl PlanDecisionEvidence {
    /// 検証済みの質問とセッションを結ぶ。
    #[must_use]
    pub const fn new(evidence: PlanApprovalEvidence, session: PlanSession) -> Self {
        Self { evidence, session }
    }
    /// 対象と内容の根拠。
    #[must_use]
    pub const fn evidence(&self) -> &PlanApprovalEvidence {
        &self.evidence
    }
    /// 呼出セッション。
    #[must_use]
    pub const fn session(&self) -> &PlanSession {
        &self.session
    }
}
