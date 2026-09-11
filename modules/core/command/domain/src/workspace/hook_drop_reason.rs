//! `.drops`へ追記する一行の理由。
use super::HookHealthError;
/// 改行を一行へ写し、元の理由を保持する値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookDropReason(String);
impl HookDropReason {
    /// 本家と同じくCRLF/LFだけを空白へ置換する。
    #[must_use]
    pub fn new(value: impl AsRef<str>) -> Self {
        Self(value.as_ref().replace("\r\n", " ").replace('\n', " "))
    }
    /// 保存する原文。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
    /// 空でない理由を受け付ける境界。
    /// # Errors
    /// 理由が空の場合。
    pub fn parse(value: &str) -> Result<Self, HookHealthError> {
        // 空白だけの理由は「理由なし」と同じ — 初回 drop は実際の失敗理由を要求する。
        let trimmed = value.trim();
        (!trimmed.is_empty())
            .then(|| Self::new(trimmed))
            .ok_or(HookHealthError::InvalidHistory)
    }
}
