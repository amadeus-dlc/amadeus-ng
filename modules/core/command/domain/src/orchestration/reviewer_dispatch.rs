//! `ReviewerDispatch` — 進行中のレビュー 1 件を名乗る差し向け記録。
use super::{ExemptPaths, ReviewedStage, ReviewedUnit};

/// 指揮者が per-unit レビュアーを呼ぶ直前に置く記録の中身
/// (`stage-protocol-reviewer.md` §12a step 1 の `<record>/.aidlc-reviewer-dispatch.json`)。
///
/// **ハーネスの入力からは分からない 4 つの事実**を運ぶ — 誰を差し向けたか、どのステージか、
/// どの Unit が対象か、その外で何を触ってよいか。記録が無ければ強制の材料が無いので、
/// 呼出しは素通しする。ステージと Unit は記録の綴りを**逐語**で持つ (upstream
/// `parseDispatchRecord` は文法を見ない — [`ReviewedStage`] / [`ReviewedUnit`])。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewerDispatch {
    reviewer: String,
    stage: ReviewedStage,
    unit: ReviewedUnit,
    exempt: ExemptPaths,
}
impl ReviewerDispatch {
    /// 記録の全材料を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        reviewer: String,
        stage: ReviewedStage,
        unit: ReviewedUnit,
        exempt: ExemptPaths,
    ) -> ReviewerDispatch {
        ReviewerDispatch {
            reviewer,
            stage,
            unit,
            exempt,
        }
    }
    /// 差し向けたレビュアーのエージェント名。
    #[must_use]
    pub fn reviewer(&self) -> &str {
        &self.reviewer
    }
    /// レビュー対象のステージ (記録の綴りのまま)。
    #[must_use]
    pub const fn stage(&self) -> &ReviewedStage {
        &self.stage
    }
    /// レビュー対象の Unit (記録の綴りのまま)。
    #[must_use]
    pub const fn unit(&self) -> &ReviewedUnit {
        &self.unit
    }
    /// Unit の外で触ってよい経路。
    #[must_use]
    pub const fn exempt(&self) -> &ExemptPaths {
        &self.exempt
    }
    /// 呼び手がこの差し向けの当人か。
    ///
    /// upstream は「エージェント名を名乗るなら一致すること、名乗らないなら
    /// scoped registration であること」を身元とする。指揮者自身の呼出しや別の
    /// サブエージェントは素通しする。
    #[must_use]
    pub fn is_dispatched(&self, agent_type: &str, scoped_registration: bool) -> bool {
        if agent_type.is_empty() {
            return scoped_registration;
        }
        agent_type == self.reviewer
    }
}

#[cfg(test)]
mod tests {
    use super::ReviewerDispatch;
    use crate::orchestration::{ExemptPaths, ReviewedStage, ReviewedUnit};

    fn dispatch() -> ReviewerDispatch {
        ReviewerDispatch::new(
            "aidlc-architecture-reviewer-agent".to_string(),
            ReviewedStage::new("functional-design".to_string()),
            ReviewedUnit::parse("u1-alpha").unwrap(),
            ExemptPaths::default(),
        )
    }

    #[test]
    fn the_named_reviewer_is_the_dispatched_one() {
        assert!(dispatch().is_dispatched("aidlc-architecture-reviewer-agent", false));
    }

    #[test]
    fn another_agent_that_names_itself_is_never_the_dispatched_one() {
        assert!(!dispatch().is_dispatched("aidlc-developer-agent", false));
        assert!(
            !dispatch().is_dispatched("aidlc-developer-agent", true),
            "名乗った名前が優先される — scoped registration では上書きされない"
        );
    }

    #[test]
    fn a_nameless_call_is_the_dispatched_one_only_under_scoped_registration() {
        assert!(dispatch().is_dispatched("", true));
        assert!(!dispatch().is_dispatched("", false));
    }
}
