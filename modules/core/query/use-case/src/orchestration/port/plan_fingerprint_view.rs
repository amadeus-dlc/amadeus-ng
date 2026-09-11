//! 現在の計画指紋の読取り表現。
/// ドメインの成功結果か拒否理由。Query側では再計算しない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanFingerprintView {
    fingerprint: Option<String>,
    error: Option<String>,
}
impl PlanFingerprintView {
    /// リードモデルの行から組む。
    #[must_use]
    pub const fn new(fingerprint: Option<String>, error: Option<String>) -> Self {
        Self { fingerprint, error }
    }
    /// 計算済み指紋。
    #[must_use]
    pub fn fingerprint(&self) -> Option<&str> {
        self.fingerprint.as_deref()
    }
    /// 拒否理由。
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}
