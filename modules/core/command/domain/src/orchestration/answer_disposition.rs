//! 受理された回答の扱い。
/// 回答記録と、reportが所有する承認選択を区別する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnswerDisposition {
    /// 質問内容を確認した受領。
    SummaryConfirmed(super::SummaryEvidence),
    /// 通常の質問への回答として記録した。
    Recorded,
    /// 承認選択の本処理はreportが担うため、人間応答を消費しない。
    ApprovalGateReportOwned,
}
