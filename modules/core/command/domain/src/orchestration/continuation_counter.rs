//! 選択したcounterと、その公開成否を区別する保存値。
use super::ContinuationGuard;

/// 公開前の観測、選択した値、確定した公開成否。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuationCounter {
    before: ContinuationGuard,
    selected: ContinuationGuard,
    published: Option<bool>,
}
impl ContinuationCounter {
    /// 完全な保存材料を構築する。要求との整合は所有集約が検査する。
    #[must_use]
    pub const fn new(
        before: ContinuationGuard,
        selected: ContinuationGuard,
        published: Option<bool>,
    ) -> Self {
        Self {
            before,
            selected,
            published,
        }
    }
    /// 保存境界の公開前観測。
    #[must_use]
    pub const fn before(&self) -> &ContinuationGuard {
        &self.before
    }
    /// 保存境界の選択値。
    #[must_use]
    pub const fn selected(&self) -> &ContinuationGuard {
        &self.selected
    }
    /// Noneは未確定、Someは実際の公開成否（待機では更新不要の成功）。
    #[must_use]
    pub const fn published(&self) -> Option<bool> {
        self.published
    }
    /// 公開失敗した選択を次回のcounterへ混ぜない。
    #[must_use]
    pub const fn effective(&self) -> &ContinuationGuard {
        if matches!(self.published, Some(false)) {
            &self.before
        } else {
            &self.selected
        }
    }
}
