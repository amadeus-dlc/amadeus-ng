//! 内容確認の提示をステージ・質問ファイルごとに保持する。
use super::{DecisionRecorded, IntentExecutionId};
use std::collections::BTreeMap;
/// 通常回答の受領とは独立した、未回答の内容確認。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingSummaryDecisions {
    entries: BTreeMap<(String, String), DecisionRecorded>,
}
impl Default for PendingSummaryDecisions {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
impl PendingSummaryDecisions {
    /// 保存された提示を復元する。
    #[must_use]
    pub fn new(entries: Vec<DecisionRecorded>) -> Self {
        Self {
            entries: entries
                .into_iter()
                .filter_map(|event| {
                    event.prompt().summary_file().map(|file| {
                        (
                            (event.prompt().stage().to_string(), file.to_string()),
                            event.clone(),
                        )
                    })
                })
                .collect(),
        }
    }
    /// 読取り側へ値を畳み込む。
    pub fn fold_left<T>(&self, initial: T, f: impl FnMut(T, &DecisionRecorded) -> T) -> T {
        self.entries.values().fold(initial, f)
    }
    pub(crate) fn record(&mut self, event: &DecisionRecorded) {
        if let Some(file) = event.prompt().summary_file() {
            self.entries.insert(
                (event.prompt().stage().to_string(), file.to_string()),
                event.clone(),
            );
        }
    }
    pub(crate) fn get(&self, stage: &str, file: &str) -> Option<&DecisionRecorded> {
        self.entries.get(&(stage.to_string(), file.to_string()))
    }
    pub(crate) fn clear(&mut self, stage: &str, file: &str) {
        self.entries.remove(&(stage.to_string(), file.to_string()));
    }
    pub(crate) fn belongs_to(&self, id: &IntentExecutionId) -> bool {
        self.entries
            .values()
            .all(|event| event.aggregate_id() == id)
    }
}
