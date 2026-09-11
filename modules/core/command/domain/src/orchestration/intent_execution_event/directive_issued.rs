//! ハーネスへの指示を発行した事実。
use crate::orchestration::{ActiveDirective, IntentExecutionEventId, IntentExecutionId};
/// 指示の内容と発行の境界を保存する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectiveIssued {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    directive: ActiveDirective,
}
impl DirectiveIssued {
    /// 識別子と発行内容を束ねる。
    #[must_use]
    pub const fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        directive: ActiveDirective,
    ) -> Self {
        Self {
            id,
            aggregate_id,
            directive,
        }
    }
    /// イベントの識別子。
    #[must_use]
    pub const fn id(&self) -> &IntentExecutionEventId {
        &self.id
    }
    /// 対象実行。
    #[must_use]
    pub const fn aggregate_id(&self) -> &IntentExecutionId {
        &self.aggregate_id
    }
    /// 発行された指示。
    #[must_use]
    pub const fn directive(&self) -> &ActiveDirective {
        &self.directive
    }
}
