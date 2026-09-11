//! 実行ごとの停止制御集約の識別子。
use super::{ContinuationError, IntentExecutionId};
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// 実行ごとの停止制御集約の識別子。
pub struct WorkflowContinuationId(String);
impl WorkflowContinuationId {
    const fn new(value: String) -> Self {
        Self(value)
    }
    /// 参照する実行から、停止制御に固有の名前空間へ写す。
    #[must_use]
    pub fn for_execution(execution: &IntentExecutionId) -> Self {
        Self::new(format!("workflow-continuation:{}", execution.as_str()))
    }
    /// 保存境界の識別子を検査する。
    /// # Errors
    /// 名前空間または実行識別子が不正。
    pub fn parse(value: &str) -> Result<Self, ContinuationError> {
        let raw = value
            .strip_prefix("workflow-continuation:")
            .ok_or(ContinuationError::InvalidIdentity)?;
        let execution =
            IntentExecutionId::parse(raw).map_err(|_| ContinuationError::InvalidIdentity)?;
        let id = Self::for_execution(&execution);
        if id.as_str() != value {
            return Err(ContinuationError::InvalidIdentity);
        }
        Ok(id)
    }
    /// 保存・Query境界で使う正準識別子。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for WorkflowContinuationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
