//! 内容確認で提示する選択肢。
/// 表記は配布プロトコルの固定文言。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryChoice {
    /// 内容を確認した。
    LooksCorrect,
    /// 内容の変更を求める。
    RequestChanges,
}
impl SummaryChoice {
    /// 固定した2択だけを受け取る。
    /// # Errors
    /// 提示した選択肢と一致しない場合。
    pub fn parse(value: &str) -> Result<Self, super::AnswerError> {
        match value {
            "Looks correct" => Ok(Self::LooksCorrect),
            "Request changes" => Ok(Self::RequestChanges),
            _ => Err(super::AnswerError::InvalidSummaryChoice),
        }
    }
}
