//! 回答を受理できない理由。
use super::CommandError;
/// 回答の意味に対応する拒否。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnswerError {
    /// 内容確認の2択と一致しない。
    InvalidSummaryChoice,
    /// 対象に対応する未回答の内容確認が無い。
    SummaryQuestionMissing,
    /// 内容確認の提示後の、未消費の人間応答が無い。
    SummaryHumanReplyMissing,
    /// 取消・無応答を示すウィジェットの結果。
    Dismissed,
    /// 未消費の人間応答が無い。
    HumanReplyMissing {
        /// 承認選択としてreportへ委ねる場面か。
        approval_choice: bool,
    },
    /// 共通のイベント記録ガードによる拒否。
    Command(CommandError),
}
impl core::fmt::Display for AnswerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidSummaryChoice => f.write_str("invalid summary choice"),
            Self::SummaryQuestionMissing => f.write_str("summary question missing"),
            Self::SummaryHumanReplyMissing => f.write_str("human reply after summary missing"),
            Self::Dismissed => f.write_str("dismissed question"),
            Self::HumanReplyMissing { approval_choice } => write!(
                f,
                "human reply missing (approval choice: {approval_choice})"
            ),
            Self::Command(error) => write!(f, "{error}"),
        }
    }
}
impl std::error::Error for AnswerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Command(error) => Some(error),
            Self::HumanReplyMissing { .. }
            | Self::Dismissed
            | Self::InvalidSummaryChoice
            | Self::SummaryQuestionMissing
            | Self::SummaryHumanReplyMissing => None,
        }
    }
}
