//! 診断実施の永続化表現。ドメインとは独立したDTO。
use super::dto_decode_error::DtoDecodeError;
use core_command_domain::orchestration::{
    HealthCheckResult, HealthChecked, IntentExecutionEventId, IntentExecutionId,
};
use serde::{Deserialize, Serialize};
/// 保存側と読取側がそれぞれ所有する診断事実の表現。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthCheckedDto {
    id: String,
    aggregate_id: String,
    passed: u64,
    failed: u64,
}
impl HealthCheckedDto {
    pub(super) fn of(event: &HealthChecked) -> Self {
        Self {
            id: event.id().as_str().to_string(),
            aggregate_id: event.aggregate_id().as_str().to_string(),
            passed: event.result().passed(),
            failed: event.result().failed(),
        }
    }
    pub(super) fn to_domain(&self) -> Result<HealthChecked, DtoDecodeError> {
        Ok(HealthChecked::new(
            IntentExecutionEventId::parse(&self.id)
                .map_err(|_| DtoDecodeError::malformed("id", &self.id))?,
            IntentExecutionId::parse(&self.aggregate_id)
                .map_err(|_| DtoDecodeError::malformed("aggregate_id", &self.aggregate_id))?,
            HealthCheckResult::new(self.passed, self.failed),
        ))
    }
}
