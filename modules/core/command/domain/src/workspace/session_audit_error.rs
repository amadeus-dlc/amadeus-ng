//! セッション監査の材料・履歴の拒否理由。
#[derive(Debug, Clone, PartialEq, Eq)]
/// セッション監査の不変条件違反。
pub enum SessionAuditError {
    /// 識別子の形式が不正。
    InvalidIdentity,
    /// イベントIDが正準UUID v7ではない。
    InvalidEventIdentity,
    /// 種別または必須監査項目が不正。
    InvalidRecord,
    /// 別の記録への混入。
    TargetMismatch,
    /// 保存履歴の不変条件違反。
    InvalidHistory,
    /// 通番の上限。
    CounterExhausted,
}
impl std::fmt::Display for SessionAuditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "session audit: {self:?}")
    }
}
impl std::error::Error for SessionAuditError {}
