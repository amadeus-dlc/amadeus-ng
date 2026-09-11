//! セッション監査の通知ID結果。
#[derive(Debug, Clone, PartialEq, Eq)]
/// 投影済みの対象・監査種別。適用判断を再計算しない。
pub struct SessionAuditView {
    id: String,
    target: String,
    kind: String,
}
impl SessionAuditView {
    /// 行の全値を構築する。
    #[must_use]
    pub const fn new(id: String, target: String, kind: String) -> Self {
        Self { id, target, kind }
    }
    /// 指定した通知ID。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// 保存時の対象。
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }
    /// 保存された監査語彙。
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }
}
