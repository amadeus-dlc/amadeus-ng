//! 未完了の指示発行に対応する、共有承認の失効準備。
use super::{PlanApprovalOperationId, PlanInvalidation};
use std::collections::BTreeMap;
/// 未完了がある間は承認を判断せず、元の操作を回復する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanInvalidations {
    entries: BTreeMap<PlanApprovalOperationId, PlanInvalidation>,
}
impl Default for PlanInvalidations {
    fn default() -> Self {
        Self::of_entries(Default::default())
    }
}
impl PlanInvalidations {
    // 検査済みの索引と空の既定値を、同じ完全な構築口へ集める。
    const fn of_entries(entries: BTreeMap<PlanApprovalOperationId, PlanInvalidation>) -> Self {
        Self { entries }
    }
    /// 記録済みの各要素を重複なく組む。
    /// # Errors
    /// 同じキーが複数ある場合。
    pub fn new(
        values: impl IntoIterator<Item = PlanInvalidation>,
    ) -> Result<Self, super::PlanApprovalError> {
        let mut entries = BTreeMap::new();
        for value in values {
            if entries.insert(value.id().clone(), value).is_some() {
                return Err(super::PlanApprovalError::new(
                    "duplicate approval collection entry",
                ));
            }
        }
        Ok(Self::of_entries(entries))
    }

    /// まだ確定していない失効準備。
    pub fn iter(&self) -> impl Iterator<Item = &PlanInvalidation> {
        self.entries.values()
    }
    /// 回復待ちがないか。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub(super) fn insert(&mut self, value: PlanInvalidation) {
        self.entries.insert(value.id().clone(), value);
    }
    pub(super) fn contains(&self, id: &PlanApprovalOperationId) -> bool {
        self.entries.contains_key(id)
    }
    pub(super) fn remove(&mut self, id: &PlanApprovalOperationId) {
        self.entries.remove(id);
    }
}
