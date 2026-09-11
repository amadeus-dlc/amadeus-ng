//! 未回答の通常質問をステージ別に保持する一級コレクション。
use super::{DecisionRecorded, IntentExecutionId};
use std::collections::BTreeMap;
/// 同じステージの後続提示は、直前の質問を置き換える。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingDecisions {
    entries: BTreeMap<String, DecisionRecorded>,
}
impl Default for PendingDecisions {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
impl PendingDecisions {
    /// 保存された提示を順序どおりに復元する。
    #[must_use]
    pub fn new(entries: Vec<DecisionRecorded>) -> Self {
        Self {
            entries: entries
                .into_iter()
                .map(|entry| (entry.prompt().stage().to_string(), entry))
                .collect(),
        }
    }
    /// 読取り側へコレクションを畳み込む。
    pub fn fold_left<T>(&self, initial: T, f: impl FnMut(T, &DecisionRecorded) -> T) -> T {
        self.entries.values().fold(initial, f)
    }
    pub(crate) fn record(&mut self, event: &DecisionRecorded) {
        self.entries
            .insert(event.prompt().stage().to_string(), event.clone());
    }
    pub(crate) fn clear(&mut self, stage: &str) {
        self.entries.remove(stage);
    }
    pub(crate) fn contains(&self, stage: &str) -> bool {
        self.entries.contains_key(stage)
    }
    pub(crate) fn belongs_to(&self, id: &IntentExecutionId) -> bool {
        self.entries
            .values()
            .all(|event| event.aggregate_id() == id)
    }
}
