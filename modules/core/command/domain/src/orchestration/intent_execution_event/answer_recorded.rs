//! 受理した回答と、その結果の事実。
use crate::orchestration::{
    AnswerDisposition, AnswerId, IntentExecutionEventId, IntentExecutionId,
};
/// 呼出側が持つAnswerIdへ結果を対応付ける。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerRecorded {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    answer_id: AnswerId,
    stage: String,
    details: String,
    disposition: AnswerDisposition,
}
impl AnswerRecorded {
    /// 受理した回答を組む。
    #[must_use]
    pub fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        answer_id: AnswerId,
        stage: impl Into<String>,
        details: impl Into<String>,
        disposition: AnswerDisposition,
    ) -> Self {
        Self {
            id,
            aggregate_id,
            answer_id,
            stage: stage.into(),
            details: details.into(),
            disposition,
        }
    }
    /// イベント識別子。
    #[must_use]
    pub const fn id(&self) -> &IntentExecutionEventId {
        &self.id
    }
    /// 対象実行。
    #[must_use]
    pub const fn aggregate_id(&self) -> &IntentExecutionId {
        &self.aggregate_id
    }
    /// 呼出側が発行した回答識別子。
    #[must_use]
    pub const fn answer_id(&self) -> &AnswerId {
        &self.answer_id
    }
    /// 対象ステージ。
    #[must_use]
    pub fn stage(&self) -> &str {
        &self.stage
    }
    /// 回答内容。
    #[must_use]
    pub fn details(&self) -> &str {
        &self.details
    }
    /// 受理結果。
    #[must_use]
    pub const fn disposition(&self) -> &AnswerDisposition {
        &self.disposition
    }
}
