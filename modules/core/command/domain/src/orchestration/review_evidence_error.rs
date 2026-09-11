//! レビュー対象の不一致。表示文言は呼出境界が選ぶ。
/// 要求・判定のどの証拠が成立しないか。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewEvidenceError {
    /// この未完了要求は許された再試行を使い切った。
    RetryAlreadyUsed,
    /// 先行するレビューの判定が未受領。
    PendingIterations(Vec<u32>),
    /// 安定した成果物集合がない。
    ArtifactsUnavailable,
    /// 要求後に追記範囲外が変わった。
    ArtifactsChanged,
    /// 要求後にソースが変わった。
    SourceChanged,
    /// 保存された結合値が不正。
    InvalidBinding,
    /// 要求前のReview節が残っている。
    StaleAppendix,
    /// Review節の検証不成立。
    InvalidAppendix(String),
}
impl std::fmt::Display for ReviewEvidenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PendingIterations(iterations) => {
                write!(f, "pending review iterations: {iterations:?}")
            }
            Self::RetryAlreadyUsed => f.write_str("review retry already used"),
            Self::ArtifactsUnavailable => f.write_str("artifacts unavailable"),
            Self::ArtifactsChanged => f.write_str("artifacts changed"),
            Self::SourceChanged => f.write_str("source changed"),
            Self::InvalidBinding => f.write_str("invalid review binding"),
            Self::StaleAppendix => f.write_str("prior appendix retained"),
            Self::InvalidAppendix(reason) => f.write_str(reason),
        }
    }
}
impl std::error::Error for ReviewEvidenceError {}
