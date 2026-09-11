//! 作業の診断を実施した事実。
use crate::orchestration::{HealthCheckResult, IntentExecutionEventId, IntentExecutionId};
/// HEALTH_CHECKEDへ投影する診断結果。作業の進行や承認を変えない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthChecked {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    result: HealthCheckResult,
}
impl HealthChecked {
    /// 対象実行と判定済み結果を束ねる。
    #[must_use]
    pub const fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        result: HealthCheckResult,
    ) -> Self {
        Self {
            id,
            aggregate_id,
            result,
        }
    }
    /// 診断事実の識別子。
    #[must_use]
    pub const fn id(&self) -> &IntentExecutionEventId {
        &self.id
    }
    /// 診断した作業実行。
    #[must_use]
    pub const fn aggregate_id(&self) -> &IntentExecutionId {
        &self.aggregate_id
    }
    /// 診断で確定した件数。
    #[must_use]
    pub const fn result(&self) -> &HealthCheckResult {
        &self.result
    }
}
