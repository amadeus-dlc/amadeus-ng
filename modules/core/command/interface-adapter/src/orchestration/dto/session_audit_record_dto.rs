//! セッション監査内容の保存表現。
use super::DtoDecodeError;
use core_command_domain::workspace::{
    AuditFieldKey, AuditFieldValue, AuditFields, EventType, SessionAuditRecord,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SessionAuditRecordDto {
    kind: String,
    fields: Vec<(String, String)>,
}
impl SessionAuditRecordDto {
    pub(crate) fn of(record: &SessionAuditRecord) -> Self {
        Self {
            kind: record.kind().as_str().into(),
            fields: record
                .fields()
                .iter()
                .map(|(key, value)| (key.as_str().into(), value.as_str().into()))
                .collect(),
        }
    }
    pub(crate) fn to_domain(&self) -> Result<SessionAuditRecord, DtoDecodeError> {
        let kind = EventType::parse(&self.kind).ok_or(DtoDecodeError::InvariantViolation)?;
        let mut fields = AuditFields::new();
        for (key, value) in &self.fields {
            if AuditFieldValue::of(value).as_str() != value {
                return Err(DtoDecodeError::InvariantViolation);
            }
            fields = fields.with(
                AuditFieldKey::parse(key).map_err(|_| DtoDecodeError::InvariantViolation)?,
                value,
            );
        }
        if fields.len() != self.fields.len() {
            return Err(DtoDecodeError::InvariantViolation);
        }
        SessionAuditRecord::new(kind, fields).map_err(|_| DtoDecodeError::InvariantViolation)
    }
}
