//! 作業の診断実施を既存の実行へ記録する。
use super::{HealthCheckError, IntentExecutionRepository};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{HealthCheckResult, IntentExecutionId};
/// 診断結果の保存だけを行う更新ユースケース。
#[derive(Debug)]
pub struct RecordHealthCheckUseCase<R: IntentExecutionRepository> {
    repository: R,
}
impl<R: IntentExecutionRepository> RecordHealthCheckUseCase<R> {
    /// 既存作業の保存ポートを注入する。
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
    /// 診断実施を保存する。呼出側は診断起動時の監査存在を先に確認する。
    /// # Errors
    /// 対象不在、保存失敗、集約の採番失敗。
    pub async fn execute(
        &mut self,
        id: &IntentExecutionId,
        result: HealthCheckResult,
        at: DateTime<Utc>,
    ) -> Result<(), HealthCheckError> {
        let mut execution = self
            .repository
            .find_by_id(id)
            .await
            .map_err(HealthCheckError::Repository)?;
        let event = execution
            .record_health_check(result, at)
            .map_err(HealthCheckError::Command)?;
        self.repository
            .store(&event, &execution)
            .await
            .map_err(HealthCheckError::Repository)
    }
}
