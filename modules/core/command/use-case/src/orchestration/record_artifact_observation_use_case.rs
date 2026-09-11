//! 成果物の保存事実をArtifactAuditへ委譲するコマンド。
use super::{ArtifactAuditCommandError, ArtifactAuditRepository};
use chrono::{DateTime, Utc};
use core_command_domain::workspace::{ArtifactAudit, ArtifactWriteObservation};
/// 媒体を直接知らない保存観測UseCase。
pub struct RecordArtifactObservationUseCase<R> {
    repository: R,
}
impl<R: ArtifactAuditRepository> RecordArtifactObservationUseCase<R> {
    /// Repositoryを注入する。
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
    /// 1件の観測事実を集約へ渡して保存する。
    /// # Errors
    /// 集約の拒否または保存失敗。
    pub async fn execute(
        mut self,
        observation: ArtifactWriteObservation,
        at: DateTime<Utc>,
    ) -> Result<(), ArtifactAuditCommandError> {
        let id = ArtifactAudit::id_for(&observation);
        let result = self.repository.find_by_id(&id).await;
        match result {
            Ok(mut aggregate) => {
                let event = aggregate.record(observation, at)?;
                self.repository.store(&event, &aggregate).await?;
            }
            Err(super::RepositoryError::NotFound { .. }) => {
                let (aggregate, event) = ArtifactAudit::start(observation, at)?;
                self.repository.store(&event, &aggregate).await?;
            }
            Err(error) => return Err(error.into()),
        }
        Ok(())
    }
}
