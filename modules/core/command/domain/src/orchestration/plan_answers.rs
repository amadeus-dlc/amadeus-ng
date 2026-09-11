//! 操作IDごとに計画回答と配送状態を保持する。
use super::{PlanAnswer, PlanAnswerState, PlanApprovalError, PlanApprovalOperationId};
use std::collections::BTreeMap;
/// 回答操作と、その完了状態の集合。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanAnswers {
    entries: BTreeMap<PlanApprovalOperationId, PlanAnswer>,
}
impl Default for PlanAnswers {
    fn default() -> Self {
        Self::of_entries(Default::default())
    }
}
impl PlanAnswers {
    // 検査済みの索引と空の既定値を、同じ完全な構築口へ集める。
    const fn of_entries(entries: BTreeMap<PlanApprovalOperationId, PlanAnswer>) -> Self {
        Self { entries }
    }
    /// 重複のない回答集合を組む。
    /// # Errors
    /// 操作IDが重複する場合。
    pub fn new(values: impl IntoIterator<Item = PlanAnswer>) -> Result<Self, PlanApprovalError> {
        let mut entries = BTreeMap::new();
        for value in values {
            if entries.insert(value.id().clone(), value).is_some() {
                return Err(PlanApprovalError::new("duplicate plan answer"));
            }
        }
        Ok(Self::of_entries(entries))
    }
    /// 特定の回答を読む。
    #[must_use]
    pub fn get(&self, id: &PlanApprovalOperationId) -> Option<&PlanAnswer> {
        self.entries.get(id)
    }
    /// 全回答の読取り。
    pub fn iter(&self) -> impl Iterator<Item = &PlanAnswer> {
        self.entries.values()
    }
    /// 配送待ちの回答。
    pub fn pending(&self) -> impl Iterator<Item = &PlanAnswer> {
        self.entries
            .values()
            .filter(|answer| matches!(answer.state(), PlanAnswerState::Pending))
    }
    pub(super) fn insert(&mut self, answer: PlanAnswer) {
        self.entries.insert(answer.id().clone(), answer);
    }
    pub(super) fn record(&mut self, id: &PlanApprovalOperationId) {
        if let Some(answer) = self.entries.get_mut(id) {
            answer.record();
        }
    }
    pub(super) fn abort(&mut self, id: &PlanApprovalOperationId, message: String) {
        if let Some(answer) = self.entries.get_mut(id) {
            answer.abort(message);
        }
    }
}
