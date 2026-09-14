//! `WorkspaceDoctor` の本家イベントストア用識別子。
use core_command_domain::workspace::WorkspaceDoctorId;
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
/// `WorkspaceDoctor` のストアキー。公開Store型の型引数に使用する。
pub struct WorkspaceDoctorAggregateKeyDto(String);
impl std::fmt::Display for WorkspaceDoctorAggregateKeyDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
/// 本家AggregateIdの公開契約に準拠する。
impl event_store_adapter_rs::types::AggregateId for WorkspaceDoctorAggregateKeyDto {
    fn type_name(&self) -> String {
        "WorkspaceDoctor".to_string()
    }
    fn value(&self) -> String {
        self.0.clone()
    }
}
impl WorkspaceDoctorAggregateKeyDto {
    pub(crate) fn of(id: &WorkspaceDoctorId) -> Self {
        Self(id.to_string())
    }
}
