//! 公開後のソースを照合し、実装開始を確定または失効する。
use super::{PlanApprovalCommandError, PlanApprovalRuntimeRepository};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{PlanApprovalOperationId, PlanApprovalRuntimeId};
/// 照合結果はイベントへ保存し、表示用の戻り値は持たない。
#[derive(Debug)]
pub struct CertifyGenerationUseCase<R> {
    repository: R,
}
impl<R: PlanApprovalRuntimeRepository> CertifyGenerationUseCase<R> {
    /// 共有集約の保存先を注入する。
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
    /// 指定操作を公開後の観測に従って確定する。
    /// # Errors
    /// 操作不一致・保存失敗。
    pub async fn execute(
        &mut self,
        id: &PlanApprovalOperationId,
        source_after: Option<&str>,
        at: DateTime<Utc>,
    ) -> Result<(), PlanApprovalCommandError> {
        let mut runtime = self
            .repository
            .find_by_id(&PlanApprovalRuntimeId::Workspace)
            .await?;
        let event = runtime.certify_generation(id, source_after, at)?;
        self.repository.store(&event, &runtime).await?;
        Ok(())
    }
}
