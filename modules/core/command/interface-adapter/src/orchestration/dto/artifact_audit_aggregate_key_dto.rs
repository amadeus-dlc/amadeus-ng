//! ArtifactAuditの本家イベントストア用識別子。
use core_command_domain::workspace::ArtifactAuditId;
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
/// ArtifactAuditのストアキー。公開Store型の型引数に使用する。
pub struct ArtifactAuditAggregateKeyDto(String);
impl std::fmt::Display for ArtifactAuditAggregateKeyDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
/// 本家AggregateIdの公開契約に準拠する。
impl event_store_adapter_rs::types::AggregateId for ArtifactAuditAggregateKeyDto {
    fn type_name(&self) -> String {
        "ArtifactAudit".to_string()
    }
    fn value(&self) -> String {
        self.0.clone()
    }
}
impl ArtifactAuditAggregateKeyDto {
    pub(crate) fn of(id: &ArtifactAuditId) -> Self {
        Self(id.to_string())
    }
}
