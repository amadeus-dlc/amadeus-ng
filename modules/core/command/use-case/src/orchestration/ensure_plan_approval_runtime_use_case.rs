//! 初めて使うワークスペースに、承認状態の所有者を作る。
use super::{PlanApprovalCommandError, PlanApprovalRuntimeRepository, RepositoryError};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{PlanApprovalRuntime, PlanApprovalRuntimeId};
/// 既存の所有者があれば変更せず、明示的な不在の場合だけ誕生を記録する。
#[derive(Debug)]
pub struct EnsurePlanApprovalRuntimeUseCase<R> {
    repository: R,
}
impl<R: PlanApprovalRuntimeRepository> EnsurePlanApprovalRuntimeUseCase<R> {
    /// 初期化する保存先を注入する。
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
    /// 誕生が必要な場合だけ、集約の誕生イベントを保存する。
    /// # Errors
    /// 破損・I/O・競合。不在以外の失敗を新規作成へ読み替えない。
    pub async fn execute(&mut self, at: DateTime<Utc>) -> Result<(), PlanApprovalCommandError> {
        match self
            .repository
            .find_by_id(&PlanApprovalRuntimeId::Workspace)
            .await
        {
            Ok(runtime) => {
                runtime.require_initialization_only()?;
                Ok(())
            }
            Err(RepositoryError::NotFound { .. }) => {
                let (runtime, event) = PlanApprovalRuntime::create(at);
                self.repository.store(&event, &runtime).await?;
                Ok(())
            }
            Err(error) => Err(error.into()),
        }
    }
}
