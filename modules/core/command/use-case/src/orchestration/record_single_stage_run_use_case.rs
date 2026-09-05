//! `RecordSingleStageRunUseCase` — 隔離実行 (`report --single`) の対を記録する（#73）。

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{IntentExecutionId, SingleStageRunRefusal};
use core_command_domain::workflow_definition::StageSlug;

use super::port::IntentExecutionRepository;
use super::port::IntentRepository;
use super::port::RepositoryError;
use super::single_stage_run_error::SingleStageRunError;

/// 隔離実行したステージの疑似ワークフロー ID 付き対を記録する
/// （`aidlc-orchestrate report --single --stage <slug>`）。
///
/// 定型は [`super::ParkUseCase`] と同じ 3 手である: **`find_by_id` で集約を再構成 →
/// 集約コマンドで判断 → `store` で保存**。
///
/// # ここに無いもの
///
/// - **業務判断**。initialization を断るか、本流の状態に依らず受理するかは
///   `IntentExecution::record_single_stage_run` が持つ。**本流を進めないこと**も同じく
///   集約側の性質である（イベントの適用がフレーム空 — 仕様 I10、オーナー裁定 2026-09-04）。
///   このユースケースが遷移ポートを持たないことではなく、集約の適用の空虚さがそれを保証する。
/// - **文言**。`Single-stage run of "<slug>" committed under …` のような逐語は合成ルートの
///   `wording` が組む。
/// - **リードモデルの更新**。`catch_up` を起動するのは合成ルートである。
///
/// # 束縛はスタティック
///
/// `dyn` は使わない（`coding-rules/use-case-rules.md` §2）。結線は合成ルートだけが行う。
#[derive(Debug)]
pub struct RecordSingleStageRunUseCase<E: IntentExecutionRepository, I: IntentRepository> {
    intent_execution_repository: E,
    intent_repository: I,
}

/// [`RecordSingleStageRunUseCase::attempt`] 1 回分の結末。
///
/// 楽観 version の競合だけを `Err` から切り出しているのは、**1 回だけ再構成からやり直す**
/// ためである（`ParkUseCase` と同じ政策）。
#[derive(Debug)]
enum AttemptOutcome {
    /// 決着した — 対をコミットした。
    Settled,
    /// 楽観 version が競合した（2 回目も競合したらこれを伝播する）。
    Conflicted(RepositoryError<IntentExecutionId>),
}

impl<E: IntentExecutionRepository, I: IntentRepository> RecordSingleStageRunUseCase<E, I> {
    /// ポートの実装を 2 つ注入する（**この型の唯一の構築経路**）。
    #[must_use]
    pub const fn new(
        intent_execution_repository: E,
        intent_repository: I,
    ) -> RecordSingleStageRunUseCase<E, I> {
        RecordSingleStageRunUseCase {
            intent_execution_repository,
            intent_repository,
        }
    }

    /// 名指しされたステージの隔離実行を記録する。
    ///
    /// `occurred_at` は呼出側が持つ時計の読みである — 集約は時計を持たない（NFR3.1）。
    ///
    /// # 戻り値を持たない理由（CQS）
    ///
    /// 状態を変えるので Command であり、CQS が定める Command の形をそのまま採る
    /// （`coding-rules/command-query-separation.md`）。記録された事実は投影後の監査台帳から読む。
    ///
    /// # `Conflict` は 1 回だけ再試行する
    ///
    /// 再試行は**再構成からやり直す** — 古い集約に `store` だけ打ち直すのは楽観ロックの
    /// 意味そのものを壊す。2 回目も `Conflict` なら伝播する。
    ///
    /// # Errors
    ///
    /// 実行の再構成・永続化の失敗（`Repository`）、計画の取得の失敗（`IntentRepository`）、
    /// 名指しの slug が計画に無い（`UnknownStage`）、集約による拒否（`Command`）を返す。
    pub async fn execute(
        &mut self,
        execution_id: &IntentExecutionId,
        stage: &StageSlug,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), SingleStageRunError> {
        if let AttemptOutcome::Settled = self.attempt(execution_id, stage, occurred_at).await? {
            return Ok(());
        }
        match self.attempt(execution_id, stage, occurred_at).await? {
            AttemptOutcome::Settled => Ok(()),
            AttemptOutcome::Conflicted(conflict) => Err(SingleStageRunError::Repository(conflict)),
        }
    }

    /// 再構成からコミットまでの 1 回分。競合したときはこれをもう 1 度だけ通す。
    async fn attempt(
        &mut self,
        execution_id: &IntentExecutionId,
        stage: &StageSlug,
        occurred_at: DateTime<Utc>,
    ) -> Result<AttemptOutcome, SingleStageRunError> {
        let mut aggregate = self
            .intent_execution_repository
            .find_by_id(execution_id)
            .await?;
        let intent = self
            .intent_repository
            .find_for_execution(&aggregate)
            .await?;
        let event = aggregate
            .record_single_stage_run(&intent, stage, occurred_at)
            .map_err(|error| match error {
                SingleStageRunRefusal::UnknownStage => SingleStageRunError::UnknownStage {
                    slug: stage.clone(),
                },
                SingleStageRunRefusal::Command(error) => SingleStageRunError::Command {
                    stage: stage.clone(),
                    error,
                },
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
            Err(other) => Err(SingleStageRunError::Repository(other)),
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
    use super::super::single_stage_run_error::SingleStageRunError;
    use super::super::test_support::{
        InMemoryIntentExecutionRepository, InMemoryIntentRepository, absent_execution, at,
        execution_id, genesis, slug,
    };
    use super::RecordSingleStageRunUseCase;
    use core_command_domain::orchestration::{
        CommandError, Intent, IntentExecution, IntentExecutionEvent,
    };
    use core_command_domain::workflow_definition::StageSlug;

    struct Subject {
        use_case: RecordSingleStageRunUseCase<
            InMemoryIntentExecutionRepository,
            InMemoryIntentRepository,
        >,
    }

    impl Subject {
        async fn execute(&mut self, stage: &StageSlug) -> Result<(), SingleStageRunError> {
            self.use_case.execute(&execution_id(), stage, at()).await
        }

        const fn intent_execution_repository(&self) -> &InMemoryIntentExecutionRepository {
            self.use_case.intent_execution_repository()
        }
    }

    fn use_case(pair: (Intent, IntentExecution), version: usize) -> Subject {
        let (intent, aggregate) = pair;
        Subject {
            use_case: RecordSingleStageRunUseCase::new(
                InMemoryIntentExecutionRepository::holding(aggregate, version),
                InMemoryIntentRepository::holding(intent),
            ),
        }
    }

    fn running(stage_count: usize) -> (Intent, IntentExecution) {
        let (intent, aggregate, _) = genesis(stage_count);
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
    async fn recording_commits_the_pair_for_the_named_stage() {
        let mut subject = use_case(running(3), 1);

        subject
            .execute(&slug(2))
            .await
            .expect("非 init は隔離実行できる");

        // 記録されるのは名指しされたステージであって、カーソルではない。
        assert!(
            matches!(
                only_committed(subject.intent_execution_repository()),
                IntentExecutionEvent::SingleStageRunCommitted(committed)
                    if committed.stage() == &slug(2)
            ),
            "名指しされたステージの SingleStageRunCommitted を期待した"
        );
    }

    #[tokio::test]
    async fn a_stage_outside_the_plan_is_this_use_cases_own_refusal() {
        let mut subject = use_case(running(3), 1);
        let absent = StageSlug::parse("nowhere").expect("文法内の slug");

        let err = subject.execute(&absent).await.expect_err("計画に無い");

        assert!(matches!(
            err,
            SingleStageRunError::UnknownStage { ref slug } if slug.as_str() == "nowhere"
        ));
        assert!(subject.intent_execution_repository().committed().is_empty());
    }

    #[tokio::test]
    async fn an_initialization_stage_is_refused_by_the_aggregate() {
        let mut subject = use_case(running(3), 1);

        let err = subject
            .execute(&slug(0))
            .await
            .expect_err("initialization は隔離実行できない");

        assert!(matches!(
            err,
            SingleStageRunError::Command {
                error: CommandError::InvalidTarget(_),
                ..
            }
        ));
        assert!(subject.intent_execution_repository().committed().is_empty());
    }

    #[tokio::test]
    async fn a_missing_aggregate_is_reported_as_not_found() {
        let (intent, _, _) = genesis(3);
        let mut use_case = RecordSingleStageRunUseCase::new(
            InMemoryIntentExecutionRepository::empty(),
            InMemoryIntentRepository::holding(intent),
        );

        let err = use_case
            .execute(&absent_execution(), &slug(1), at())
            .await
            .expect_err("ストアに無い集約は再構成できない");

        assert!(matches!(
            err,
            SingleStageRunError::Repository(RepositoryError::NotFound { id }) if id == absent_execution()
        ));
    }

    #[tokio::test]
    async fn a_missing_intent_is_propagated_from_its_own_port() {
        let (intent, aggregate) = running(3);
        let mut use_case = RecordSingleStageRunUseCase::new(
            InMemoryIntentExecutionRepository::holding(aggregate, 7),
            InMemoryIntentRepository::empty(),
        );

        let err = use_case
            .execute(&execution_id(), &slug(1), at())
            .await
            .expect_err("計画が引けなければ記録できない");

        assert!(matches!(
            err,
            SingleStageRunError::IntentRepository(RepositoryError::NotFound { id }) if id == *intent.id()
        ));
    }

    #[tokio::test]
    async fn a_first_conflict_is_retried_once_from_the_rehydration() {
        let (intent, aggregate) = running(3);
        let mut subject = Subject {
            use_case: RecordSingleStageRunUseCase::new(
                InMemoryIntentExecutionRepository::holding_behind_concurrent_writes(
                    aggregate, 7, 1,
                ),
                InMemoryIntentRepository::holding(intent),
            ),
        };

        subject
            .execute(&slug(2))
            .await
            .expect("1 回だけ再試行すれば通る");

        assert!(matches!(
            only_committed(subject.intent_execution_repository()),
            IntentExecutionEvent::SingleStageRunCommitted(_)
        ));
        assert_eq!(
            subject.intent_execution_repository().store_attempts(),
            2,
            "再試行は 1 回だけ"
        );
    }

    #[tokio::test]
    async fn a_second_conflict_is_propagated_without_a_further_retry() {
        let (intent, aggregate) = running(3);
        let mut subject = Subject {
            use_case: RecordSingleStageRunUseCase::new(
                InMemoryIntentExecutionRepository::holding_behind_concurrent_writes(
                    aggregate, 7, 2,
                ),
                InMemoryIntentRepository::holding(intent),
            ),
        };

        let err = subject
            .execute(&slug(2))
            .await
            .expect_err("2 回目も競合したら伝播する");

        assert!(matches!(
            err,
            SingleStageRunError::Repository(RepositoryError::Conflict {
                expected: 8,
                actual: 9,
            })
        ));
        assert!(subject.intent_execution_repository().committed().is_empty());
    }
}
