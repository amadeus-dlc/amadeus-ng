//! 通常指示の発行前に、共有承認の失効準備を保存する。
use super::{PlanApprovalCommandError, PlanApprovalRuntimeRepository};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{PlanApprovalRuntimeId, PlanInvalidation};
/// コマンドは結果材料を返さず、呼出側の操作IDで後から投影を読む。
#[derive(Debug)]
pub struct PreparePlanInvalidationUseCase<R> {
    repository: R,
}
impl<R: PlanApprovalRuntimeRepository> PreparePlanInvalidationUseCase<R> {
    /// 集約Repositoryを注入する。
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
    /// 準備の事実を集約で確定して保存する。
    /// # Errors
    /// 集約の拒否・読取・保存の失敗。
    pub async fn execute(
        &mut self,
        invalidation: PlanInvalidation,
        at: DateTime<Utc>,
    ) -> Result<(), PlanApprovalCommandError> {
        let mut runtime = self
            .repository
            .find_by_id(&PlanApprovalRuntimeId::Workspace)
            .await?;
        let event = runtime.prepare_invalidation(invalidation, at)?;
        self.repository.store(&event, &runtime).await?;
        Ok(())
    }
}
