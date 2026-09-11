//! コマンドが失敗したという事実。
use crate::orchestration::{CommandFailure, IntentExecutionEventId, IntentExecutionId};

/// ERROR_LOGGEDへ投影する失敗記録。実行許可は発行しない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandFailed {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    failure: CommandFailure,
}
impl CommandFailed {
    /// 識別子と失敗の観測を束ねる。
    #[must_use]
    pub const fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        failure: CommandFailure,
    ) -> Self {
        Self {
            id,
            aggregate_id,
            failure,
        }
    }
    /// 事実自身の識別子。
    #[must_use]
    pub const fn id(&self) -> &IntentExecutionEventId {
        &self.id
    }
    /// 対象の実行。
    #[must_use]
    pub const fn aggregate_id(&self) -> &IntentExecutionId {
        &self.aggregate_id
    }
    /// 投影する失敗の観測。
    #[must_use]
    pub const fn failure(&self) -> &CommandFailure {
        &self.failure
    }
}
