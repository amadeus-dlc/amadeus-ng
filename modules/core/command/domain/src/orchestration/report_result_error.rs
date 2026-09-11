//! 報告結果の構築拒否。
/// 適用した操作と遷移事実が一致しない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportResultError {
    /// 操作列と遷移の不一致。
    TransitionMismatch,
    /// 新たな工程へ進まない報告にソース基準を付けた。
    BaselineWithoutAdvance,
}
impl std::fmt::Display for ReportResultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TransitionMismatch => f.write_str("report transition mismatch"),
            Self::BaselineWithoutAdvance => f.write_str("source baseline without stage advance"),
        }
    }
}
impl std::error::Error for ReportResultError {}
