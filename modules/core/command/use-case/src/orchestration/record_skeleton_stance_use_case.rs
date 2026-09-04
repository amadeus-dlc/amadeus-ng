//! `RecordSkeletonStanceUseCase` — conductor が分類した walking-skeleton stance を記録する（#73）。

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{IntentExecutionId, SkeletonStance};

use super::port::IntentExecutionRepository;
use super::port::IntentRepository;
use super::port::RepositoryError;
use super::skeleton_stance_error::SkeletonStanceError;

/// 分類された stance を記録する（`aidlc-orchestrate report --skeleton-stance <値>`）。
///
/// 定型は [`super::ParkUseCase`] と同じ 3 手である: **`find_by_id` で集約を再構成 →
/// 集約コマンドで判断 → `store` で保存**。
///
/// # ここに無いもの
///
/// - **業務判断**。「いまが skeleton-gate ステージか」も「再記録を受理するか」も
///   `IntentExecution::record_skeleton_stance` が持つ。
/// - **文言**。`Recorded walking-skeleton stance …` のような逐語は合成ルートの `wording` が組む。
/// - **リードモデルの更新**。`catch_up` を起動するのは合成ルートである。
#[derive(Debug)]
pub struct RecordSkeletonStanceUseCase<E: IntentExecutionRepository, I: IntentRepository> {
    intent_execution_repository: E,
    intent_repository: I,
}

/// [`RecordSkeletonStanceUseCase::attempt`] 1 回分の結末。
#[derive(Debug)]
enum AttemptOutcome {
    /// 決着した — stance をコミットした。
    Settled,
    /// 楽観 version が競合した（2 回目も競合したらこれを伝播する）。
    Conflicted(RepositoryError<IntentExecutionId>),
}

impl<E: IntentExecutionRepository, I: IntentRepository> RecordSkeletonStanceUseCase<E, I> {
    /// ポートの実装を 2 つ注入する（**この型の唯一の構築経路**）。
    #[must_use]
    pub const fn new(
        intent_execution_repository: E,
        intent_repository: I,
    ) -> RecordSkeletonStanceUseCase<E, I> {
        RecordSkeletonStanceUseCase {
            intent_execution_repository,
            intent_repository,
        }
    }

    /// 分類された stance を記録する。
    ///
    /// `occurred_at` は呼出側が持つ時計の読みである — 集約は時計を持たない（NFR3.1）。
    ///
    /// # Errors
    ///
    /// 実行の再構成・永続化の失敗（`Repository`）、計画の取得の失敗（`IntentRepository`）、
    /// 集約による拒否（`Command` — 現在地が skeleton-gate ステージでない）を返す。
    pub async fn execute(
        &mut self,
        execution_id: &IntentExecutionId,
        stance: SkeletonStance,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), SkeletonStanceError> {
        if let AttemptOutcome::Settled = self.attempt(execution_id, stance, occurred_at).await? {
            return Ok(());
        }
        match self.attempt(execution_id, stance, occurred_at).await? {
            AttemptOutcome::Settled => Ok(()),
            AttemptOutcome::Conflicted(conflict) => Err(SkeletonStanceError::Repository(conflict)),
        }
    }

    /// 再構成からコミットまでの 1 回分。競合したときはこれをもう 1 度だけ通す。
    async fn attempt(
        &mut self,
        execution_id: &IntentExecutionId,
        stance: SkeletonStance,
        occurred_at: DateTime<Utc>,
    ) -> Result<AttemptOutcome, SkeletonStanceError> {
        let mut aggregate = self
            .intent_execution_repository
            .find_by_id(execution_id)
            .await?;
        let intent = self
            .intent_repository
            .find_by_id(aggregate.intent_id())
            .await?;
        // 拒否の文言の材料は「そのときの現在地」と scope である — 判断ではなく観測なので、
        // コマンドを打つ前に控えておく。
        let cursor = aggregate.cursor_slug().cloned();
        let scope = intent.scope().to_string();
        let event = aggregate
            .record_skeleton_stance(&intent, stance, occurred_at)
            .map_err(|error| SkeletonStanceError::Command {
                stage: cursor,
                scope,
                error,
            })?;
        match self
            .intent_execution_repository
            .store(&event, &aggregate)
            .await
        {
            Ok(()) => Ok(AttemptOutcome::Settled),
            Err(conflict @ RepositoryError::Conflict { .. }) => {
                Ok(AttemptOutcome::Conflicted(conflict))
            }
            Err(other) => Err(SkeletonStanceError::Repository(other)),
        }
    }

    /// 注入された実行ポートの実装（テストが**効果**を観測するための継ぎ目）。
    #[cfg(test)]
    pub(crate) const fn intent_execution_repository(&self) -> &E {
        &self.intent_execution_repository
    }
}

#[cfg(test)]
mod tests {
    use super::super::port::RepositoryError;
    use super::super::skeleton_stance_error::SkeletonStanceError;
    use super::super::test_support::{
        InMemoryIntentExecutionRepository, InMemoryIntentRepository, absent_execution, at,
        execution_id, genesis, skeleton_gate_plan, slug,
    };
    use super::RecordSkeletonStanceUseCase;
    use core_command_domain::orchestration::{
        CommandError, Intent, IntentExecution, IntentExecutionEvent, SkeletonStance,
    };

    struct Subject {
        use_case: RecordSkeletonStanceUseCase<
            InMemoryIntentExecutionRepository,
            InMemoryIntentRepository,
        >,
    }

    impl Subject {
        async fn execute(&mut self, stance: SkeletonStance) -> Result<(), SkeletonStanceError> {
            self.use_case.execute(&execution_id(), stance, at()).await
        }

        const fn intent_execution_repository(&self) -> &InMemoryIntentExecutionRepository {
            self.use_case.intent_execution_repository()
        }
    }

    fn use_case(pair: (Intent, IntentExecution), version: usize) -> Subject {
        let (intent, aggregate) = pair;
        Subject {
            use_case: RecordSkeletonStanceUseCase::new(
                InMemoryIntentExecutionRepository::holding(aggregate, version),
                InMemoryIntentRepository::holding(intent),
            ),
        }
    }

    /// カーソルが skeleton-gate ステージ（索引 1 = Construction の最初の EXECUTE）に立つ実行。
    fn at_the_skeleton_gate() -> (Intent, IntentExecution) {
        let (intent, aggregate, _) = skeleton_gate_plan();
        (intent, aggregate)
    }

    fn only_committed(
        intent_execution_repository: &InMemoryIntentExecutionRepository,
    ) -> &IntentExecutionEvent {
        let committed = intent_execution_repository.committed();
        assert_eq!(committed.len(), 1, "コミットは 1 件のはず");
        committed.first().expect("1 件ある")
    }

    #[tokio::test]
    async fn recording_commits_the_classified_stance() {
        let mut subject = use_case(at_the_skeleton_gate(), 1);

        subject
            .execute(SkeletonStance::ScopeDependent)
            .await
            .expect("skeleton-gate では記録できる");

        assert!(
            matches!(
                only_committed(subject.intent_execution_repository()),
                IntentExecutionEvent::SkeletonStanceRecorded(recorded)
                    if recorded.stance() == SkeletonStance::ScopeDependent
            ),
            "分類された stance の SkeletonStanceRecorded を期待した"
        );
    }

    #[tokio::test]
    async fn a_stance_away_from_the_skeleton_gate_is_refused_with_the_cursor_and_scope() {
        // 索引 1 以降が inception の計画には skeleton-gate ステージが無い。
        let (intent, aggregate, _) = genesis(3);
        let mut subject = use_case((intent, aggregate), 1);

        let err = subject
            .execute(SkeletonStance::On)
            .await
            .expect_err("skeleton-gate でなければ拒む");

        // 拒否の材料は「拒否された時点の現在地」と scope である。
        assert!(
            matches!(
                &err,
                SkeletonStanceError::Command { stage, scope, error }
                    if *stage == Some(slug(1))
                        && scope == "classic"
                        && matches!(error, CommandError::InvalidTarget(_))
            ),
            "現在地と scope を運ぶ Command を期待した: {err:?}"
        );
        assert!(subject.intent_execution_repository().committed().is_empty());
    }

    #[tokio::test]
    async fn a_missing_aggregate_is_reported_as_not_found() {
        let (intent, _) = at_the_skeleton_gate();
        let mut use_case = RecordSkeletonStanceUseCase::new(
            InMemoryIntentExecutionRepository::empty(),
            InMemoryIntentRepository::holding(intent),
        );

        let err = use_case
            .execute(&absent_execution(), SkeletonStance::On, at())
            .await
            .expect_err("ストアに無い集約は再構成できない");

        assert!(matches!(
            err,
            SkeletonStanceError::Repository(RepositoryError::NotFound { id }) if id == absent_execution()
        ));
    }

    #[tokio::test]
    async fn a_missing_intent_is_propagated_from_its_own_port() {
        let (intent, aggregate) = at_the_skeleton_gate();
        let mut use_case = RecordSkeletonStanceUseCase::new(
            InMemoryIntentExecutionRepository::holding(aggregate, 7),
            InMemoryIntentRepository::empty(),
        );

        let err = use_case
            .execute(&execution_id(), SkeletonStance::On, at())
            .await
            .expect_err("計画が引けなければ記録できない");

        assert!(matches!(
            err,
            SkeletonStanceError::IntentRepository(RepositoryError::NotFound { id }) if id == *intent.id()
        ));
    }

    #[tokio::test]
    async fn a_first_conflict_is_retried_once_from_the_rehydration() {
        let (intent, aggregate) = at_the_skeleton_gate();
        let mut subject = Subject {
            use_case: RecordSkeletonStanceUseCase::new(
                InMemoryIntentExecutionRepository::holding_behind_concurrent_writes(
                    aggregate, 7, 1,
                ),
                InMemoryIntentRepository::holding(intent),
            ),
        };

        subject
            .execute(SkeletonStance::On)
            .await
            .expect("1 回だけ再試行すれば通る");

        assert!(matches!(
            only_committed(subject.intent_execution_repository()),
            IntentExecutionEvent::SkeletonStanceRecorded(_)
        ));
        assert_eq!(
            subject.intent_execution_repository().store_attempts(),
            2,
            "再試行は 1 回だけ"
        );
    }

    #[tokio::test]
    async fn a_second_conflict_is_propagated_without_a_further_retry() {
        let (intent, aggregate) = at_the_skeleton_gate();
        let mut subject = Subject {
            use_case: RecordSkeletonStanceUseCase::new(
                InMemoryIntentExecutionRepository::holding_behind_concurrent_writes(
                    aggregate, 7, 2,
                ),
                InMemoryIntentRepository::holding(intent),
            ),
        };

        let err = subject
            .execute(SkeletonStance::On)
            .await
            .expect_err("2 回目も競合したら伝播する");

        assert!(matches!(
            err,
            SkeletonStanceError::Repository(RepositoryError::Conflict {
                expected: 8,
                actual: 9,
            })
        ));
        assert!(subject.intent_execution_repository().committed().is_empty());
    }
}
