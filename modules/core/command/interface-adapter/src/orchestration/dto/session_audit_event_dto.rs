//! セッション監査イベントの保存DTO。
use super::{DtoDecodeError, SessionAuditRecordDto};
use core_command_domain::workspace::{
    HookHealthTarget, SessionAuditEvent, SessionAuditEventId, SessionAuditId,
    SessionAuditObservationId,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// 公開Repositoryのストア型に使う、単一事実の保存表現。
pub struct SessionAuditEventDto {
    id: String,
    observation_id: String,
    aggregate_id: String,
    target: String,
    record: SessionAuditRecordDto,
}
impl SessionAuditEventDto {
    pub(crate) fn of(event: &SessionAuditEvent) -> Self {
        Self {
            id: event.id().to_string(),
            observation_id: event.observation_id().to_string(),
            aggregate_id: event.aggregate_id().to_string(),
            target: event.target().relative_directory(),
            record: SessionAuditRecordDto::of(event.record()),
        }
    }
    pub(crate) fn to_domain(&self) -> Result<SessionAuditEvent, DtoDecodeError> {
        SessionAuditEvent::new(
            SessionAuditEventId::parse(&self.id).map_err(|_| DtoDecodeError::InvariantViolation)?,
            SessionAuditObservationId::parse(&self.observation_id)
                .map_err(|_| DtoDecodeError::InvariantViolation)?,
            SessionAuditId::parse(&self.aggregate_id)
                .map_err(|_| DtoDecodeError::InvariantViolation)?,
            HookHealthTarget::parse(&self.target)
                .map_err(|_| DtoDecodeError::InvariantViolation)?,
            self.record.to_domain()?,
        )
        .map_err(|_| DtoDecodeError::InvariantViolation)
    }
}
