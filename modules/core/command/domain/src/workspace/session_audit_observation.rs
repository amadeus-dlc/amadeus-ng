//! 実フックで観測した監査材料。workflowの集約写しは保持しない。
use super::{HookHealthTarget, SessionAuditRecord};
#[derive(Debug, Clone, PartialEq, Eq)]
/// 実フックから得た、対象と状態の観測。
pub struct SessionAuditObservation {
    id: super::SessionAuditObservationId,
    target: HookHealthTarget,
    record: SessionAuditRecord,
    workflow_status: String,
}
impl SessionAuditObservation {
    /// 対象・観測項目・状態ファイル上のstatusを同時に構築する。
    #[must_use]
    pub const fn new(
        id: super::SessionAuditObservationId,
        target: HookHealthTarget,
        record: SessionAuditRecord,
        workflow_status: String,
    ) -> Self {
        Self {
            id,
            target,
            record,
            workflow_status,
        }
    }
    /// 呼出側が指定した通知ID。
    #[must_use]
    pub const fn id(&self) -> &super::SessionAuditObservationId {
        &self.id
    }
    /// 対象となる記録。
    #[must_use]
    pub const fn target(&self) -> &HookHealthTarget {
        &self.target
    }
    /// 記録する観測。
    #[must_use]
    pub const fn record(&self) -> &SessionAuditRecord {
        &self.record
    }
    /// 読み取ったworkflow status。
    #[must_use]
    pub fn workflow_status(&self) -> &str {
        &self.workflow_status
    }
}
