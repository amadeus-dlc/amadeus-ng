//! RMU自身が所有するSessionAuditの読込DTO。
use core_command_domain::workspace::{
    AuditFieldKey, AuditFieldValue, AuditFields, EventType, HookHealthTarget, SessionAuditEvent,
    SessionAuditEventId, SessionAuditId, SessionAuditObservationId, SessionAuditRecord,
};
use serde::Deserialize;
#[derive(Deserialize)]
struct RecordDto {
    kind: String,
    fields: Vec<(String, String)>,
}
#[derive(Deserialize)]
pub(super) struct SessionEventDto {
    id: String,
    observation_id: String,
    aggregate_id: String,
    target: String,
    record: RecordDto,
}
impl SessionEventDto {
    pub(super) fn decode(bytes: &[u8], aggregate: &str) -> Result<SessionAuditEvent, ()> {
        let wire: Self = serde_json::from_slice(bytes).map_err(|_| ())?;
        if wire.aggregate_id != aggregate {
            return Err(());
        }
        let kind = EventType::parse(&wire.record.kind).ok_or(())?;
        let mut fields = AuditFields::new();
        for (key, value) in &wire.record.fields {
            if AuditFieldValue::of(value).as_str() != value {
                return Err(());
            }
            fields = fields.with(AuditFieldKey::parse(key).map_err(|_| ())?, value);
        }
        if fields.len() != wire.record.fields.len() {
            return Err(());
        }
        SessionAuditEvent::new(
            SessionAuditEventId::parse(&wire.id).map_err(|_| ())?,
            SessionAuditObservationId::parse(&wire.observation_id).map_err(|_| ())?,
            SessionAuditId::parse(&wire.aggregate_id).map_err(|_| ())?,
            HookHealthTarget::parse(&wire.target).map_err(|_| ())?,
            SessionAuditRecord::new(kind, fields).map_err(|_| ())?,
        )
        .map_err(|_| ())
    }
}
