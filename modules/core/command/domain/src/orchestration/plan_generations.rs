//! 実装開始操作と認証状態の集合。
use super::{PlanApprovalError, PlanApprovalOperationId, PlanGeneration, PlanGenerationState};
use std::collections::BTreeMap;
/// 操作IDで一意に保持する開始履歴。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanGenerations {
    entries: BTreeMap<PlanApprovalOperationId, PlanGeneration>,
}
impl Default for PlanGenerations {
    fn default() -> Self {
        Self::of_entries(Default::default())
    }
}
impl PlanGenerations {
    // 検査済みの索引と空の既定値を、同じ完全な構築口へ集める。
    const fn of_entries(entries: BTreeMap<PlanApprovalOperationId, PlanGeneration>) -> Self {
        Self { entries }
    }
    /// 重複のない集合を組む。
    /// # Errors
    /// 操作IDが重複する場合。
    pub fn new(
        values: impl IntoIterator<Item = PlanGeneration>,
    ) -> Result<Self, PlanApprovalError> {
        let mut entries = BTreeMap::new();
        for value in values {
            if entries.insert(value.id().clone(), value).is_some() {
                return Err(PlanApprovalError::new("duplicate generation operation"));
            }
        }
        Ok(Self::of_entries(entries))
    }
    /// 指定操作の読取り。
    #[must_use]
    pub fn get(&self, id: &PlanApprovalOperationId) -> Option<&PlanGeneration> {
        self.entries.get(id)
    }
    /// 全履歴。
    pub fn iter(&self) -> impl Iterator<Item = &PlanGeneration> {
        self.entries.values()
    }
    /// ソース再照合待ちの開始操作。
    pub fn pending(&self) -> impl Iterator<Item = &PlanGeneration> {
        self.entries
            .values()
            .filter(|entry| entry.state() == PlanGenerationState::Pending)
    }
    pub(super) fn apply_certification(
        &mut self,
        event: &super::PlanGenerationCertified,
    ) -> Result<(), super::PlanRuntimeError> {
        self.entries
            .get_mut(event.operation_id())
            .ok_or(super::PlanRuntimeError::NoPendingGeneration)?
            .apply_certification(event)
    }
    pub(super) fn apply_revocation(
        &mut self,
        event: &super::PlanGenerationRevoked,
    ) -> Result<(), super::PlanRuntimeError> {
        self.entries
            .get_mut(event.operation_id())
            .ok_or(super::PlanRuntimeError::NoPendingGeneration)?
            .apply_revocation(event)
    }
    pub(super) fn insert(&mut self, value: PlanGeneration) {
        self.entries.insert(value.id().clone(), value);
    }
}
