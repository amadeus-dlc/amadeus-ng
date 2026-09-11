//! フックの最初または後続のdropを一つの事実として保存する。
use super::{HookHealthCommandError, HookHealthRepository, RepositoryError};
use chrono::{DateTime, Utc};
use core_command_domain::workspace::{HookHealth, HookHealthId, HookHealthTarget, HookName};
/// heartbeatの有無に依存せず失敗観測を記録する更新ユースケース。
#[derive(Debug)]
pub struct RecordHookDropUseCase<R: HookHealthRepository> {
    repository: R,
}
impl<R: HookHealthRepository> RecordHookDropUseCase<R> {
    /// 同じHookHealth保存ポートを注入する。
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
    /// 観測を保存する。競合時は同じ対象・理由・時刻で一回だけ再試行する。
    /// # Errors
    /// 理由が不正、履歴が枯渇、または保存できない場合。
    pub async fn execute(
        &mut self,
        target: &HookHealthTarget,
        hook: &HookName,
        reason: &str,
        at: DateTime<Utc>,
    ) -> Result<(), HookHealthCommandError> {
        match self.attempt(target, hook, reason, at).await {
            Err(HookHealthCommandError::Repository(RepositoryError::Conflict { .. })) => {
                self.attempt(target, hook, reason, at).await
            }
            result => result,
        }
    }
    async fn attempt(
        &mut self,
        target: &HookHealthTarget,
        hook: &HookName,
        reason: &str,
        at: DateTime<Utc>,
    ) -> Result<(), HookHealthCommandError> {
        let id = HookHealthId::for_hook(target, hook);
        let (health, event) = match self.repository.find_by_id(&id).await {
            Ok(mut health) => {
                let event = health
                    .record_drop(reason, at)
                    .map_err(HookHealthCommandError::Domain)?;
                (health, event)
            }
            Err(RepositoryError::NotFound { .. }) => {
                HookHealth::start_with_drop(target.clone(), hook.clone(), reason, at)
                    .map_err(HookHealthCommandError::Domain)?
            }
            Err(error) => return Err(HookHealthCommandError::Repository(error)),
        };
        self.repository
            .store(&event, &health)
            .await
            .map_err(HookHealthCommandError::Repository)
    }
}
