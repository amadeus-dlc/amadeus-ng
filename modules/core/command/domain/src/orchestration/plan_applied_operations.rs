//! 共有承認で確定済みの操作識別子。
use super::PlanApprovalOperationId;
use std::collections::BTreeSet;
/// 失効や質問の差替えを越えて、同じ操作の再実行を防ぐ集合。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanAppliedOperations {
    entries: BTreeSet<PlanApprovalOperationId>,
}
impl Default for PlanAppliedOperations {
    fn default() -> Self {
        Self::of_entries(Default::default())
    }
}
impl PlanAppliedOperations {
    // 検査済みの索引と空の既定値を、同じ完全な構築口へ集める。
    const fn of_entries(entries: BTreeSet<PlanApprovalOperationId>) -> Self {
        Self { entries }
    }
    /// 記録済みの各要素を重複なく組む。
    /// # Errors
    /// 同じキーが複数ある場合。
    pub fn new(
        values: impl IntoIterator<Item = PlanApprovalOperationId>,
    ) -> Result<Self, super::PlanApprovalError> {
        let mut entries = BTreeSet::new();
        for value in values {
            if !entries.insert(value) {
                return Err(super::PlanApprovalError::new(
                    "duplicate approval collection entry",
                ));
            }
        }
        Ok(Self::of_entries(entries))
    }

    /// 確定済みの各識別子。
    pub fn iter(&self) -> impl Iterator<Item = &PlanApprovalOperationId> {
        self.entries.iter()
    }
    /// この操作が既に確定したか。
    #[must_use]
    pub fn contains(&self, id: &PlanApprovalOperationId) -> bool {
        self.entries.contains(id)
    }
    pub(super) fn insert(&mut self, id: PlanApprovalOperationId) {
        self.entries.insert(id);
    }
}
