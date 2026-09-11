//! SessionAuditの本家イベントストア用識別子。
use core_command_domain::workspace::SessionAuditId;
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
/// SessionAuditのストアキー。公開Store型の型引数に使用する。
pub struct SessionAuditKeyDto(String);
impl std::fmt::Display for SessionAuditKeyDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
/// 本家AggregateIdの公開契約に準拠する。
impl event_store_adapter_rs::types::AggregateId for SessionAuditKeyDto {
    fn type_name(&self) -> String {
        "SessionAudit".to_string()
    }
    fn value(&self) -> String {
        self.0.clone()
    }
}
impl SessionAuditKeyDto {
    pub(crate) fn of(id: &SessionAuditId) -> Self {
        Self(id.to_string())
    }
}
