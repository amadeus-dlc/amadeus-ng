//! 停止要求を判断し、反復回数を確定した事実。
use super::{ContinuationRequest, WorkflowContinuationEventId, WorkflowContinuationId};
#[derive(Debug, Clone, PartialEq, Eq)]
/// 停止要求を判断し、反復回数を確定した事実。
pub struct WorkflowContinuationEvent {
    id: WorkflowContinuationEventId,
    aggregate_id: WorkflowContinuationId,
    request: ContinuationRequest,
    count: u64,
    blocked: bool,
    publication: Option<bool>,
}
impl WorkflowContinuationEvent {
    /// 保存境界から全事実を復号する。
    /// # Errors
    /// 回数・保存された判断が要求上限と矛盾する場合。
    pub fn new(
        id: WorkflowContinuationEventId,
        aggregate_id: WorkflowContinuationId,
        request: ContinuationRequest,
        count: u64,
        blocked: bool,
    ) -> Result<Self, super::ContinuationError> {
        if request.wait().is_none()
            && !request.is_wait_probe()
            && (count == 0) != request.is_reset()
            || blocked
                != (request.wait().is_none()
                    && !request.is_wait_probe()
                    && !request.is_reset()
                    && count < request.limit())
        {
            return Err(super::ContinuationError::InvalidHistory);
        }
        Ok(Self {
            id,
            aggregate_id,
            request,
            count,
            blocked,
            publication: None,
        })
    }
    /// 同じ要求の公開成否を、選択イベントと区別して記録する。
    #[must_use]
    pub const fn with_publication(mut self, succeeded: bool) -> Self {
        self.publication = Some(succeeded);
        self
    }
    /// Noneは停止判断、Someは公開確定イベント。
    #[must_use]
    pub const fn publication(&self) -> Option<bool> {
        self.publication
    }
    /// 事実自身の識別子。
    #[must_use]
    pub const fn id(&self) -> &WorkflowContinuationEventId {
        &self.id
    }
    /// 所有する停止制御集約。
    #[must_use]
    pub const fn aggregate_id(&self) -> &WorkflowContinuationId {
        &self.aggregate_id
    }
    /// 判断した要求。
    #[must_use]
    pub const fn request(&self) -> &ContinuationRequest {
        &self.request
    }
    /// 確定した反復回数。
    #[must_use]
    pub const fn count(&self) -> u64 {
        self.count
    }
    /// 確定した停止差止め判断。
    #[must_use]
    pub const fn blocked(&self) -> bool {
        self.blocked
    }
}
