//! Stopが選定した質問文書と、ハーネスの会話観測。
use super::ContinuationQuestions;
/// 外部から読んだ事実。待機可否の状態判断はIntentExecutionが持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuationObservations {
    questions: ContinuationQuestions,
    conversational: bool,
    resume_waiting: bool,
}
impl ContinuationObservations {
    /// 状態hashと照合済みの共有再開promptを伴う。
    #[must_use]
    pub const fn with_resume_waiting(mut self, waiting: bool) -> Self {
        self.resume_waiting = waiting;
        self
    }
    /// 正当な共有再開promptを観測したか。
    #[must_use]
    pub const fn is_resume_waiting(&self) -> bool {
        self.resume_waiting
    }

    /// 全観測材料を束ねる。
    #[must_use]
    pub const fn new(questions: ContinuationQuestions, conversational: bool) -> Self {
        Self {
            questions,
            conversational,
            resume_waiting: false,
        }
    }
    /// 選定した文書に未回答タグがあるか。
    #[must_use]
    pub fn has_unanswered_question(&self) -> bool {
        self.questions.has_unanswered()
    }
    /// ハーネスの最終人間prompt後にengine呼出しがないという観測。
    #[must_use]
    pub const fn is_conversational(&self) -> bool {
        self.conversational
    }
}
