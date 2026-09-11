//! 停止要求を集約へ委譲し、単一事実として保存する。
use super::{
    ContinuationCommandError, IntentExecutionRepository, RepositoryError,
    WorkflowContinuationRepository,
};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    ContinuationRequest, WorkflowContinuation, WorkflowContinuationId,
};
/// 停止要求を集約へ委譲し、単一事実として保存する。
pub struct RecordContinuationUseCase<R, E> {
    repository: R,
    executions: E,
}
impl<R: WorkflowContinuationRepository, E: IntentExecutionRepository>
    RecordContinuationUseCase<R, E>
{
    /// Repositoryを注入する。
    #[must_use]
    pub const fn new(repository: R, executions: E) -> Self {
        Self {
            repository,
            executions,
        }
    }
    /// 成功はunitのみ。停止判断はRMUを経た要求ID Queryで読む。
    /// # Errors
    /// 入力拒否、履歴不整合、保存障害。
    pub async fn execute(
        mut self,
        execution_id: &core_command_domain::orchestration::IntentExecutionId,
        request: ContinuationRequest,
        raw_limit: Option<&str>,
        observations: &core_command_domain::orchestration::ContinuationObservations,
        at: DateTime<Utc>,
    ) -> Result<(), ContinuationCommandError> {
        let execution = self
            .executions
            .find_by_id(execution_id)
            .await
            .map_err(ContinuationCommandError::ExecutionRepository)?;
        let request = execution.prepare_continuation(request, raw_limit, observations)?;
        let id = WorkflowContinuationId::for_execution(execution_id);
        match self.repository.find_by_id(&id).await {
            Ok(mut aggregate) => {
                let event = aggregate.consider(request, at)?;
                self.repository.store(&event, &aggregate).await?;
            }
            Err(RepositoryError::NotFound { .. }) => {
                let (aggregate, event) = WorkflowContinuation::start(id.clone(), request, at)?;
                self.repository.store(&event, &aggregate).await?;
            }
            Err(error) => return Err(error.into()),
        }
        Ok(())
    }
}
