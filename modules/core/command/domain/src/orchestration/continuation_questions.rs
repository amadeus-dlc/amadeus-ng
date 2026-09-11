//! 選定したステージ直下で読めた質問文書の集合。
use super::ContinuationQuestion;
/// 他ステージ・他Unitの文書を含めない質問観測。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuationQuestions {
    questions: Vec<ContinuationQuestion>,
}
impl ContinuationQuestions {
    /// 選定済み文書だけを束ねる。
    #[must_use]
    pub const fn new(questions: Vec<ContinuationQuestion>) -> Self {
        Self { questions }
    }
    /// 未回答の文書が一つでもあるか。
    #[must_use]
    pub fn has_unanswered(&self) -> bool {
        self.questions
            .iter()
            .any(ContinuationQuestion::is_unanswered)
    }
}
