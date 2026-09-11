//! 通常回答の入力。
/// 更新側へ渡す構文検証済みの入力。受理判断は集約に置く。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerRequest {
    summary: Option<super::SummaryEvidence>,
    stage: String,
    details: String,
    human_presence_guard: bool,
}

impl AnswerRequest {
    pub(crate) fn is_non_answer(&self) -> bool {
        let text = self.details.trim().to_ascii_lowercase();
        if text.is_empty() {
            return true;
        }
        let text = text
            .strip_suffix('.')
            .or_else(|| text.strip_suffix('!'))
            .unwrap_or(&text);
        matches!(
            text,
            "cancel"
                | "cancelled"
                | "canceled"
                | "cancellation"
                | "dismiss"
                | "dismissed"
                | "abort"
                | "aborted"
                | "time out"
                | "time-out"
                | "timeout"
                | "timed out"
                | "timed-out"
                | "timedout"
                | "no answer"
                | "no response"
                | "user cancelled"
                | "user canceled"
                | "user dismissed"
                | "question cancelled"
                | "question canceled"
                | "question dismissed"
        )
    }
    /// 対象と回答、テスト用ガード観測を束ねる。
    #[must_use]
    pub fn new(
        stage: impl Into<String>,
        details: impl Into<String>,
        human_presence_guard: bool,
    ) -> Self {
        Self {
            summary: None,
            stage: stage.into(),
            details: details.into(),
            human_presence_guard,
        }
    }
    /// 確認した質問内容の検証結果を渡す。
    #[must_use]
    pub fn with_summary(mut self, evidence: super::SummaryEvidence) -> Self {
        self.summary = Some(evidence);
        self
    }
    /// 内容確認の入力。
    #[must_use]
    pub const fn summary(&self) -> Option<&super::SummaryEvidence> {
        self.summary.as_ref()
    }

    /// 対象ステージ。
    #[must_use]
    pub fn stage(&self) -> &str {
        &self.stage
    }
    /// 回答の内容。
    #[must_use]
    pub fn details(&self) -> &str {
        &self.details
    }
    /// 人間応答の検査が有効か。
    #[must_use]
    pub const fn human_presence_guard(&self) -> bool {
        self.human_presence_guard
    }
}

#[cfg(test)]
mod tests {
    use super::AnswerRequest;
    #[test]
    fn punctuation_and_substantive_cancellation_instructions_are_answers() {
        for details in [".", "!", "Cancelled!!", "cancel the standing order"] {
            assert!(
                !AnswerRequest::new("requirements-analysis", details, true).is_non_answer(),
                "{details}"
            );
        }
    }
}
