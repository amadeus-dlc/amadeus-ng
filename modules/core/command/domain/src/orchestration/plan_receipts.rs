//! 現在の対象・発行エポックに対応する保護された受領。
use super::{PlanApprovalError, PlanApprovalReceipt};
use std::collections::BTreeMap;
/// 現在の承認対象ごとの受領集合。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanReceipts {
    entries: BTreeMap<String, PlanApprovalReceipt>,
}
impl Default for PlanReceipts {
    fn default() -> Self {
        Self::of_entries(Default::default())
    }
}
impl PlanReceipts {
    // 検査済みの索引と空の既定値を、同じ完全な構築口へ集める。
    const fn of_entries(entries: BTreeMap<String, PlanApprovalReceipt>) -> Self {
        Self { entries }
    }
    /// 同じ受領キーが重複しない集合を組む。
    /// # Errors
    /// キー重複の場合。
    pub fn new(
        values: impl IntoIterator<Item = PlanApprovalReceipt>,
    ) -> Result<Self, PlanApprovalError> {
        let mut entries = BTreeMap::new();
        for value in values {
            if entries.insert(value.key(), value).is_some() {
                return Err(PlanApprovalError::new("duplicate approval receipt"));
            }
        }
        Ok(Self::of_entries(entries))
    }
    /// 現在の受領を読む。
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&PlanApprovalReceipt> {
        self.entries.get(key)
    }
    /// 全受領の読取り。
    pub fn iter(&self) -> impl Iterator<Item = &PlanApprovalReceipt> {
        self.entries.values()
    }
    pub(super) fn insert(&mut self, value: PlanApprovalReceipt) {
        self.entries.insert(value.key(), value);
    }
    pub(super) fn remove(&mut self, key: &str) {
        self.entries.remove(key);
    }
    pub(super) fn clear(&mut self) {
        self.entries.clear();
    }
}
