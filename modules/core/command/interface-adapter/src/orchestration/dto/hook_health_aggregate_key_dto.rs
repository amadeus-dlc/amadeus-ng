//! HookHealthの本家イベントストア用識別子。
use core_command_domain::workspace::HookHealthId;
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
/// HookHealthのストアキー。公開Store型の型引数に使用する。
pub struct HookHealthAggregateKeyDto(String);
impl std::fmt::Display for HookHealthAggregateKeyDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
/// 本家AggregateIdの公開契約に準拠する。
impl event_store_adapter_rs::types::AggregateId for HookHealthAggregateKeyDto {
    fn type_name(&self) -> String {
        "HookHealth".to_string()
    }
    fn value(&self) -> String {
        self.0.clone()
    }
}
impl HookHealthAggregateKeyDto {
    pub(crate) fn of(id: &HookHealthId) -> Self {
        Self(id.to_string())
    }
}
