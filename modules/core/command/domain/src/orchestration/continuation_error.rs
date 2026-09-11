//! 停止制御の入力・履歴の不整合。
#[derive(Debug, Clone, PartialEq, Eq)]
/// 停止制御の入力・履歴の不整合。
pub enum ContinuationError {
    /// 識別子が正準形ではない。
    InvalidIdentity,
    /// 進捗署名の構造が不正。
    InvalidSignature,
    /// 連続停止回数の上限が0。
    InvalidLimit,
    /// 保存済みの回数・通番・識別子が不整合。
    InvalidHistory,
    /// 回数または通番の上限。
    CounterExhausted,
}
impl std::fmt::Display for ContinuationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "continuation: {self:?}")
    }
}
impl std::error::Error for ContinuationError {}
