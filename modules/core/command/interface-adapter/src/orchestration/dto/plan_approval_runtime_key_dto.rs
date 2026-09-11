//! 共有承認ストリームの、イベントストア側の鍵。
use core_command_domain::orchestration::PlanApprovalRuntimeId;
use event_store_adapter_rs::types::AggregateId;
use serde::{Deserialize, Serialize};
/// ドメインへストアtraitを持ち込まないための境界型。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlanApprovalRuntimeKeyDto(String);
impl PlanApprovalRuntimeKeyDto {
    /// 共有集約の識別子を写す。
    #[must_use]
    pub fn of(id: &PlanApprovalRuntimeId) -> Self {
        Self(id.to_string())
    }
}
impl std::fmt::Display for PlanApprovalRuntimeKeyDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl AggregateId for PlanApprovalRuntimeKeyDto {
    fn type_name(&self) -> String {
        "PlanApprovalRuntime".to_string()
    }
    fn value(&self) -> String {
        self.0.clone()
    }
}
