//! 人間へ提示した通常の質問。

/// 質問の内容。承認の受領や人間が回答した証拠とは別の値である。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionPrompt {
    plan_approval: Option<Box<super::PlanDecisionEvidence>>,
    summary_file: Option<String>,
    stage: String,
    decision: String,
    options: Option<String>,
    rationale: Option<String>,
}
impl DecisionPrompt {
    /// 計画承認の質問の検証結果を結び付ける。
    #[must_use]
    pub fn with_plan_approval(mut self, evidence: super::PlanDecisionEvidence) -> Self {
        self.summary_file = None;
        self.plan_approval = Some(Box::new(evidence));
        self
    }
    /// 計画承認の質問である場合の根拠。
    #[must_use]
    pub fn plan_approval(&self) -> Option<&super::PlanDecisionEvidence> {
        self.plan_approval.as_deref()
    }

    /// 提示先と質問を保持する。CLIの必須フラグの検査は入力境界が担う。
    #[must_use]
    pub fn new(stage: impl Into<String>, decision: impl Into<String>) -> Self {
        Self {
            plan_approval: None,
            summary_file: None,
            stage: stage.into(),
            decision: decision.into(),
            options: None,
            rationale: None,
        }
    }
    /// 内容確認の質問ファイルへ結び付ける。
    #[must_use]
    pub fn with_summary_file(mut self, file: impl Into<String>) -> Self {
        self.plan_approval = None;
        self.summary_file = Some(file.into());
        self
    }
    /// 内容確認の対象ファイル。
    #[must_use]
    pub fn summary_file(&self) -> Option<&str> {
        self.summary_file.as_deref()
    }

    /// 提示した選択肢。
    #[must_use]
    pub fn with_options(mut self, options: impl Into<String>) -> Self {
        self.options = Some(options.into());
        self
    }
    /// 選択を求める根拠。
    #[must_use]
    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = Some(rationale.into());
        self
    }
    /// 対象ステージの公開綴り。
    #[must_use]
    pub fn stage(&self) -> &str {
        &self.stage
    }
    /// 質問本文。
    #[must_use]
    pub fn decision(&self) -> &str {
        &self.decision
    }
    /// 選択肢の元の文字列。
    #[must_use]
    pub fn options(&self) -> Option<&str> {
        self.options.as_deref()
    }
    /// 質問の根拠。
    #[must_use]
    pub fn rationale(&self) -> Option<&str> {
        self.rationale.as_deref()
    }
}
