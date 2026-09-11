//! セッション監査の適用を集約へ委譲し、事実だけを保存する。
use super::{RepositoryError, SessionAuditCommandError, SessionAuditRepository};
use chrono::{DateTime, Utc};
use core_command_domain::workspace::{SessionAudit, SessionAuditObservation};
/// セッション監査の保存だけを行う更新ユースケース。
pub struct RecordSessionAuditUseCase<R> {
    repository: R,
}
impl<R: SessionAuditRepository> RecordSessionAuditUseCase<R> {
    /// Repositoryを注入する。
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
    /// 成功はunit。監査文面はRMUが保存事実から描く。
    /// # Errors
    /// 観測拒否、保存失敗、履歴破損。
    pub async fn execute(
        mut self,
        observation: &SessionAuditObservation,
        at: DateTime<Utc>,
    ) -> Result<(), SessionAuditCommandError> {
        let id = SessionAudit::id_for(observation);
        match self.repository.find_by_id(&id).await {
            Ok(mut aggregate) => {
                if let Some(event) = aggregate.record(observation, at)? {
                    self.repository.store(&event, &aggregate).await?;
                }
            }
            Err(RepositoryError::NotFound { .. }) => {
                if let Some((aggregate, event)) = SessionAudit::start(observation, at)? {
                    self.repository.store(&event, &aggregate).await?;
                }
            }
            Err(error) => return Err(error.into()),
        }
        Ok(())
    }
}
