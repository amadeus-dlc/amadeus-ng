//! 計画承認の回答を受領するための、検証済み文書と観測値。
use super::{PlanApprovalOrigin, PlanChoice, PlanDecisionEvidence};
/// 回答時に照合する不変の入力値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanAnswerInput {
    origin: PlanApprovalOrigin,
    stage: String,
    decision: PlanDecisionEvidence,
    choice: PlanChoice,
    source: Option<String>,
}
impl PlanAnswerInput {
    /// 呼出時点の材料を固定する。
    #[must_use]
    pub const fn new(
        origin: PlanApprovalOrigin,
        stage: String,
        decision: PlanDecisionEvidence,
        choice: PlanChoice,
        source: Option<String>,
    ) -> Self {
        Self {
            origin,
            stage,
            decision,
            choice,
            source,
        }
    }
    /// 回答の記録先。
    #[must_use]
    pub const fn origin(&self) -> &PlanApprovalOrigin {
        &self.origin
    }
    /// 公開のステージ名。
    #[must_use]
    pub fn stage(&self) -> &str {
        &self.stage
    }
    /// 対象文書とセッションの検証結果。
    #[must_use]
    pub const fn decision(&self) -> &PlanDecisionEvidence {
        &self.decision
    }
    /// 受領する正規の選択。
    #[must_use]
    pub const fn choice(&self) -> PlanChoice {
        self.choice
    }
    /// 認証直前に観測したソース指紋。
    #[must_use]
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }
}
