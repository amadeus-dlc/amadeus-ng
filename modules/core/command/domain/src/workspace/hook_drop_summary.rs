//! HookHealthが保持する監査dropの集計。
use super::{HookDropReason, HookHealthError};
/// 件数と最新理由を一体で保持する値オブジェクト。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookDropSummary {
    count: usize,
    latest: Option<HookDropReason>,
}
impl HookDropSummary {
    /// 保存済みの集計を検査して組む。
    /// # Errors
    /// 件数と最新理由の有無が一致しない場合。
    pub fn new(count: usize, latest: Option<HookDropReason>) -> Result<Self, HookHealthError> {
        if count == 0 && latest.is_some() || count > 0 && latest.is_none() {
            return Err(HookHealthError::InvalidHistory);
        }
        Ok(Self { count, latest })
    }
    /// dropを1件適用した集計を返す。
    /// # Errors
    /// 件数が上限に達した場合。
    pub fn recorded(&self, reason: HookDropReason) -> Result<Self, HookHealthError> {
        let count = self
            .count
            .checked_add(1)
            .ok_or(HookHealthError::CounterExhausted)?;
        Self::new(count, Some(reason))
    }
    /// 件数。
    #[must_use]
    pub const fn count(&self) -> usize {
        self.count
    }
    /// 最新理由。
    #[must_use]
    pub const fn latest(&self) -> Option<&HookDropReason> {
        self.latest.as_ref()
    }
}
