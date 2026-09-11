//! SessionAuditの完全snapshot保存DTO。
use super::{DtoDecodeError, SessionAuditRecordDto};
use chrono::{DateTime, Utc};
use core_command_domain::workspace::{
    HookHealthTarget, SessionAudit, SessionAuditId, SessionAuditObservationId,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// 公開Repositoryのストア型に使うsnapshot表現。
pub struct SessionAuditDto {
    id: String,
    target: String,
    record: SessionAuditRecordDto,
    observation_id: String,
    seq_nr: usize,
    version: usize,
    occurred_at: DateTime<Utc>,
}
impl SessionAuditDto {
    pub(crate) fn of(aggregate: &SessionAudit) -> Self {
        Self {
            id: aggregate.id().to_string(),
            target: aggregate.target().relative_directory(),
            record: SessionAuditRecordDto::of(aggregate.last_record()),
            observation_id: aggregate.observation_id().to_string(),
            seq_nr: aggregate.seq_nr(),
            version: aggregate.version(),
            occurred_at: aggregate.occurred_at(),
        }
    }
    pub(crate) fn to_domain(&self) -> Result<SessionAudit, DtoDecodeError> {
        SessionAudit::new(
            SessionAuditId::parse(&self.id).map_err(|_| DtoDecodeError::InvariantViolation)?,
            HookHealthTarget::parse(&self.target)
                .map_err(|_| DtoDecodeError::InvariantViolation)?,
            self.record.to_domain()?,
            SessionAuditObservationId::parse(&self.observation_id)
                .map_err(|_| DtoDecodeError::InvariantViolation)?,
            self.seq_nr,
            self.version,
            self.occurred_at,
        )
        .map_err(|_| DtoDecodeError::InvariantViolation)
    }
}
