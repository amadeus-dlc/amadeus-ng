//! 単独pipelineの開始を一度だけ保存する更新UseCase。
use super::{IntentExecutionRepository, IntentRepository, SingleStageRunError};
use chrono::{DateTime, Utc};
use core_command_domain::{
    orchestration::{CommandError, IntentExecutionId},
    workflow_definition::StageSlug,
};
/// 既に開いている試行は保持する。
#[derive(Debug)]
pub struct BeginSingleStageRunUseCase<E: IntentExecutionRepository, I: IntentRepository> {
    executions: E,
    intents: I,
}
impl<E: IntentExecutionRepository, I: IntentRepository> BeginSingleStageRunUseCase<E, I> {
    /// 保存ポートを注入する。
    #[must_use]
    pub const fn new(executions: E, intents: I) -> Self {
        Self {
            executions,
            intents,
        }
    }
    /// 開始を記録し、成功値は返さない。
    /// # Errors
    /// 読込・保存失敗または集約の拒否。競合時は1回だけ再試行する。
    pub async fn execute(
        &mut self,
        id: &IntentExecutionId,
        stage: &StageSlug,
        at: DateTime<Utc>,
    ) -> Result<(), SingleStageRunError> {
        match self.attempt(id, stage, at).await {
            Err(SingleStageRunError::Repository(super::RepositoryError::Conflict { .. })) => {
                self.attempt(id, stage, at).await
            }
            result => result,
        }
    }
    async fn attempt(
        &mut self,
        id: &IntentExecutionId,
        stage: &StageSlug,
        at: DateTime<Utc>,
    ) -> Result<(), SingleStageRunError> {
        let mut execution = self.executions.find_by_id(id).await?;
        let intent = self.intents.find_for_execution(&execution).await?;
        let event = match execution.begin_single_stage_run(&intent, stage, at) {
            Ok(event) => event,
            Err(CommandError::SingleStageAttemptAlreadyOpen) => return Ok(()),
            Err(error) => {
                return Err(SingleStageRunError::Command {
                    stage: stage.clone(),
                    error,
                });
            }
        };
        self.executions
            .store(&event, &execution)
            .await
            .map_err(SingleStageRunError::Repository)
    }
}

#[cfg(test)]
mod tests {
    use super::super::port::RepositoryError;
    use super::super::single_stage_run_error::SingleStageRunError;
    use super::super::test_support::{
        InMemoryIntentExecutionRepository, InMemoryIntentRepository, at, execution_id, genesis,
        slug,
    };
    use super::BeginSingleStageRunUseCase;
    use core_command_domain::orchestration::{CommandError, IntentExecutionEvent};

    type Subject =
        BeginSingleStageRunUseCase<InMemoryIntentExecutionRepository, InMemoryIntentRepository>;

    #[tokio::test]
    async fn opening_a_gated_stage_commits_one_started_event() {
        let (intent, aggregate, _) = genesis(3);
        let mut subject: Subject = BeginSingleStageRunUseCase::new(
            InMemoryIntentExecutionRepository::holding(aggregate, 1),
            InMemoryIntentRepository::holding(intent),
        );
        subject
            .execute(&execution_id(), &slug(2), at())
            .await
            .expect("非 init は開ける");
        assert!(matches!(
            subject.executions.committed(),
            [IntentExecutionEvent::SingleStageRunStarted(started)] if started.stage() == &slug(2)
        ));
    }

    #[tokio::test]
    async fn an_already_open_attempt_is_kept_without_a_second_commit() {
        let (intent, mut aggregate, _) = genesis(3);
        aggregate
            .begin_single_stage_run(&intent, &slug(2), at())
            .expect("先に開いておく");
        let mut subject: Subject = BeginSingleStageRunUseCase::new(
            InMemoryIntentExecutionRepository::holding(aggregate, 1),
            InMemoryIntentRepository::holding(intent),
        );
        subject
            .execute(&execution_id(), &slug(2), at())
            .await
            .expect("既に開いている試行は成功として保持する");
        assert!(subject.executions.committed().is_empty());
    }

    #[tokio::test]
    async fn an_initialization_stage_is_refused_with_the_aggregate_rejection() {
        let (intent, aggregate, _) = genesis(3);
        let mut subject: Subject = BeginSingleStageRunUseCase::new(
            InMemoryIntentExecutionRepository::holding(aggregate, 1),
            InMemoryIntentRepository::holding(intent),
        );
        let error = subject
            .execute(&execution_id(), &slug(0), at())
            .await
            .expect_err("initialization は隔離実行できない");
        assert!(matches!(
            error,
            SingleStageRunError::Command {
                ref stage,
                error: CommandError::InvalidTarget(_),
            } if stage == &slug(0)
        ));
        assert!(subject.executions.committed().is_empty());
    }

    #[tokio::test]
    async fn a_first_conflict_is_retried_once_and_a_second_is_propagated() {
        let (intent, aggregate, _) = genesis(3);
        let mut retried: Subject = BeginSingleStageRunUseCase::new(
            InMemoryIntentExecutionRepository::holding_behind_concurrent_writes(
                aggregate.clone(),
                7,
                1,
            ),
            InMemoryIntentRepository::holding(intent.clone()),
        );
        retried
            .execute(&execution_id(), &slug(1), at())
            .await
            .expect("1 回だけ再試行すれば通る");
        assert_eq!(retried.executions.store_attempts(), 2);

        let mut exhausted: Subject = BeginSingleStageRunUseCase::new(
            InMemoryIntentExecutionRepository::holding_behind_concurrent_writes(aggregate, 7, 2),
            InMemoryIntentRepository::holding(intent),
        );
        let error = exhausted
            .execute(&execution_id(), &slug(1), at())
            .await
            .expect_err("2 回目の競合は伝播する");
        assert!(matches!(
            error,
            SingleStageRunError::Repository(RepositoryError::Conflict { .. })
        ));
    }
}
