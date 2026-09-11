//! 計画承認のために入力境界で読み取った文書。
/// 読書きの機構を持たず、計画・指示・質問の原文と正規の質問パスを運ぶ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanApprovalDocuments {
    plan: String,
    instructions: String,
    questions: String,
    questions_file: String,
}
impl PlanApprovalDocuments {
    /// 正規の配置から読み取った原文を束ねる。
    #[must_use]
    pub const fn new(
        plan: String,
        instructions: String,
        questions: String,
        questions_file: String,
    ) -> Self {
        Self {
            plan,
            instructions,
            questions,
            questions_file,
        }
    }
    /// 計画の原文。
    #[must_use]
    pub fn plan(&self) -> &str {
        &self.plan
    }
    /// テスト指示の原文。
    #[must_use]
    pub fn instructions(&self) -> &str {
        &self.instructions
    }
    /// 質問の原文。
    #[must_use]
    pub fn questions(&self) -> &str {
        &self.questions
    }
    /// 正規の質問文書のプロジェクト相対パス。
    #[must_use]
    pub fn questions_file(&self) -> &str {
        &self.questions_file
    }
}
