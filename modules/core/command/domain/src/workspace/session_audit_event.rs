//! セッション監査を記録した単一の事実。
use super::{
    HookHealthTarget, SessionAuditError, SessionAuditEventId, SessionAuditId, SessionAuditRecord,
};
#[derive(Debug, Clone, PartialEq, Eq)]
/// 一回のセッション監査を保存した事実。
pub struct SessionAuditEvent {
    id: SessionAuditEventId,
    observation_id: super::SessionAuditObservationId,
    aggregate_id: SessionAuditId,
    target: HookHealthTarget,
    record: SessionAuditRecord,
}
impl SessionAuditEvent {
    /// 保存境界の全材料を検査する。
    /// # Errors
    /// 集約IDと対象が対応しない場合。
    pub fn new(
        id: SessionAuditEventId,
        observation_id: super::SessionAuditObservationId,
        aggregate_id: SessionAuditId,
        target: HookHealthTarget,
        record: SessionAuditRecord,
    ) -> Result<Self, SessionAuditError> {
        if aggregate_id != SessionAuditId::for_target(&target) {
            return Err(SessionAuditError::TargetMismatch);
        }
        Ok(Self {
            id,
            observation_id,
            aggregate_id,
            target,
            record,
        })
    }
    /// 対応する実通知のID。
    #[must_use]
    pub const fn observation_id(&self) -> &super::SessionAuditObservationId {
        &self.observation_id
    }
    /// 事実自身のID。
    #[must_use]
    pub const fn id(&self) -> &SessionAuditEventId {
        &self.id
    }
    /// 所有集約。
    #[must_use]
    pub const fn aggregate_id(&self) -> &SessionAuditId {
        &self.aggregate_id
    }
    /// 監査の帰属先。
    #[must_use]
    pub const fn target(&self) -> &HookHealthTarget {
        &self.target
    }
    /// 保存された監査内容。
    #[must_use]
    pub const fn record(&self) -> &SessionAuditRecord {
        &self.record
    }
}
