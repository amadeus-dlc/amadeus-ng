//! 内容確認用の質問文書の拒否理由。
/// 書式と回答値の不整合。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SummaryQuestionsError {
    /// 対象節の回答行が一意でない、または期待する値と異なる。
    InvalidAnswer,
    /// 確認対象の構造が不正。
    InvalidStructure(String),
}
impl core::fmt::Display for SummaryQuestionsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidAnswer => f.write_str("summary answer mismatch"),
            Self::InvalidStructure(reason) => f.write_str(reason),
        }
    }
}
impl std::error::Error for SummaryQuestionsError {}
