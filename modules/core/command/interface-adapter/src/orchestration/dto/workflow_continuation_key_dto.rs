//! 停止制御ストアの集約キー。
use core_command_domain::orchestration::WorkflowContinuationId;
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
/// 保存先の型引数に使う停止制御IDのDTO。
#[doc(hidden)]
pub struct WorkflowContinuationKeyDto(String);
impl std::fmt::Display for WorkflowContinuationKeyDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl event_store_adapter_rs::types::AggregateId for WorkflowContinuationKeyDto {
    fn type_name(&self) -> String {
        "WorkflowContinuation".to_string()
    }
    fn value(&self) -> String {
        self.0.clone()
    }
}
impl WorkflowContinuationKeyDto {
    pub(crate) fn of(id: &WorkflowContinuationId) -> Self {
        Self(id.to_string())
    }
}
