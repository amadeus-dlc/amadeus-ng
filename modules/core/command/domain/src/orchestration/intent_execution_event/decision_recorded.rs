//! 通常の質問を人間へ提示した事実。
use crate::orchestration::{DecisionPrompt, IntentExecutionEventId, IntentExecutionId};

/// DECISION_RECORDEDへ投影する質問の事実。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionRecorded {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    prompt: DecisionPrompt,
    human_before: Option<IntentExecutionEventId>,
}
impl DecisionRecorded {
    /// 識別子と質問の内容から組む。
    #[must_use]
    pub const fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        prompt: DecisionPrompt,
    ) -> Self {
        Self {
            id,
            aggregate_id,
            prompt,
            human_before: None,
        }
    }
    /// 提示より前の人間応答を区別する。
    #[must_use]
    pub fn with_human_before(mut self, id: Option<IntentExecutionEventId>) -> Self {
        self.human_before = id;
        self
    }
    /// 提示時点の直前の人間応答。
    #[must_use]
    pub const fn human_before(&self) -> Option<&IntentExecutionEventId> {
        self.human_before.as_ref()
    }

    /// イベントの識別子。
    #[must_use]
    pub const fn id(&self) -> &IntentExecutionEventId {
        &self.id
    }
    /// 実行の識別子。
    #[must_use]
    pub const fn aggregate_id(&self) -> &IntentExecutionId {
        &self.aggregate_id
    }
    /// 質問の内容。
    #[must_use]
    pub const fn prompt(&self) -> &DecisionPrompt {
        &self.prompt
    }
}
