//! 実行側の記録と共有側の反映がまだそろっていない応答。
use super::{PlanApprovalError, PlanApprovalOperationId, PlanResponsePreparation};
use std::collections::BTreeMap;
/// 応答原文と発行回を保持して、同じ操作を回復する集合。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanPendingResponses {
    entries: BTreeMap<PlanApprovalOperationId, PlanResponsePreparation>,
}
impl Default for PlanPendingResponses {
    fn default() -> Self {
        Self::of_entries(Default::default())
    }
}
impl PlanPendingResponses {
    // 検査済みの索引と空の既定値を、同じ完全な構築口へ集める。
    const fn of_entries(
        entries: BTreeMap<PlanApprovalOperationId, PlanResponsePreparation>,
    ) -> Self {
        Self { entries }
    }
    /// 記録された準備から重複なく再構成する。
    /// # Errors
    /// 同じ操作IDが重複した場合。
    pub fn new(
        values: impl IntoIterator<Item = PlanResponsePreparation>,
    ) -> Result<Self, PlanApprovalError> {
        let mut entries = BTreeMap::new();
        for value in values {
            if entries.insert(value.id().clone(), value).is_some() {
                return Err(PlanApprovalError::new("duplicate pending response"));
            }
        }
        Ok(Self::of_entries(entries))
    }
    /// 未完了の各応答。
    pub fn iter(&self) -> impl Iterator<Item = &PlanResponsePreparation> {
        self.entries.values()
    }
    /// 未完了の応答がないか。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// 特定の操作を読む。
    #[must_use]
    pub fn get(&self, id: &PlanApprovalOperationId) -> Option<&PlanResponsePreparation> {
        self.entries.get(id)
    }
    pub(super) fn insert(&mut self, value: PlanResponsePreparation) {
        self.entries.insert(value.id().clone(), value);
    }
    pub(super) fn remove(&mut self, id: &PlanApprovalOperationId) {
        self.entries.remove(id);
    }
}
