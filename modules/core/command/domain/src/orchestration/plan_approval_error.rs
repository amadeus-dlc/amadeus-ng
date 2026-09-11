//! 計画承認の対象や根拠が成立しない理由。
/// 公開契約の拒否文言を保持する。受領成功と混ぜない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanApprovalError {
    message: String,
}
impl PlanApprovalError {
    pub(super) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
impl std::fmt::Display for PlanApprovalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for PlanApprovalError {}
