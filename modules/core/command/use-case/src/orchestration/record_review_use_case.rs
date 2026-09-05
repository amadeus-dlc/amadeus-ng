//! `RecordReviewUseCase` — レビュー受領証の対を記録する（`aidlc-log review`、b48 / B10）。

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{Intent, IntentExecutionId, IntentReviewError};
use core_command_domain::workflow_definition::ReviewPolicy;

use super::port::IntentExecutionRepository;
use super::port::IntentRepository;
use super::port::RepositoryError;
use super::port::WorkflowDefinitionRepository;
use super::review_log_error::ReviewLogError;
use super::review_log_kind::ReviewLogKind;
use super::review_log_outcome::ReviewLogOutcome;
use super::review_log_request::ReviewLogRequest;

/// レビュアーの差し向けと判定を記録する（`aidlc-log review [--verdict <v>]`）。
///
/// 定型は [`super::RecordSkeletonStanceUseCase`] と同じ 3 手である: **`find_by_id` で集約を
/// 再構成 → 集約コマンドで判断 → `store` で保存**。違うのは、レビュー方針
/// （[`ReviewPolicy`]）が定義側の静的材料なので**定義も引く**という点だけである。
///
/// # ここに無いもの
///
/// - **業務判断**。「レビュアーが宣言と一致するか」も「予算を超えていないか」も
///   「通し番号が順序どおりか」も、すべて `IntentExecution::request_review` /
///   `record_review_verdict` が持つ。
/// - **文言**。`Refusing REVIEW_REQUESTED for "<slug>": …` のような逐語は合成ルートが組む。
/// - **リードモデルの更新**。`catch_up` を起動するのは合成ルートである。
#[derive(Debug)]
pub struct RecordReviewUseCase<
    E: IntentExecutionRepository,
    I: IntentRepository,
    D: WorkflowDefinitionRepository,
> {
    intent_execution_repository: E,
    intent_repository: I,
    workflow_definition_repository: D,
}

/// [`RecordReviewUseCase::attempt`] 1 回分の結末。
#[derive(Debug)]
enum AttemptOutcome {
    /// 決着した — 受領証の行をコミットした。
    Settled(ReviewLogOutcome),
    /// 楽観 version が競合した（2 回目も競合したらこれを伝播する）。
    Conflicted(RepositoryError<IntentExecutionId>),
}

impl<E: IntentExecutionRepository, I: IntentRepository, D: WorkflowDefinitionRepository>
    RecordReviewUseCase<E, I, D>
{
    /// ポートの実装を 3 つ注入する（**この型の唯一の構築経路**）。
    #[must_use]
    pub const fn new(
        intent_execution_repository: E,
        intent_repository: I,
        workflow_definition_repository: D,
    ) -> RecordReviewUseCase<E, I, D> {
        RecordReviewUseCase {
            intent_execution_repository,
            intent_repository,
            workflow_definition_repository,
        }
    }

    /// 受領証の行を 1 つ記録する。
    ///
    /// `occurred_at` は呼出側が持つ時計の読みである — 集約は時計を持たない（NFR3.1）。
    ///
    /// # Errors
    ///
    /// 実行・intent・定義の再構成や永続化の失敗、定義がその slug を知らない
    /// （`UnknownStage`）、壊れた `--review` 値（`CorruptReviewOverride`）、集約による拒否
    /// （`Command`）を返す。
    pub async fn execute(
        &mut self,
        execution_id: &IntentExecutionId,
        request: &ReviewLogRequest,
        occurred_at: DateTime<Utc>,
    ) -> Result<ReviewLogOutcome, ReviewLogError> {
        match self.attempt(execution_id, request, occurred_at).await? {
            AttemptOutcome::Settled(outcome) => Ok(outcome),
            AttemptOutcome::Conflicted(_) => {
                match self.attempt(execution_id, request, occurred_at).await? {
                    AttemptOutcome::Settled(outcome) => Ok(outcome),
                    AttemptOutcome::Conflicted(conflict) => {
                        Err(ReviewLogError::Repository(conflict))
                    }
                }
            }
        }
    }

    /// 再構成からコミットまでの 1 回分。競合したときはこれをもう 1 度だけ通す。
    async fn attempt(
        &mut self,
        execution_id: &IntentExecutionId,
        request: &ReviewLogRequest,
        occurred_at: DateTime<Utc>,
    ) -> Result<AttemptOutcome, ReviewLogError> {
        let mut aggregate = self
            .intent_execution_repository
            .find_by_id(execution_id)
            .await?;
        let intent = self
            .intent_repository
            .find_for_execution(&aggregate)
            .await?;
        let policy = self.review_policy(&intent, request).await?;
        let refused = match request.kind() {
            ReviewLogKind::Request { retry_pending } => aggregate.request_review(
                &intent,
                request.stage(),
                policy.as_ref(),
                request.reviewer(),
                request.iteration(),
                retry_pending,
                occurred_at,
            ),
            ReviewLogKind::Verdict(verdict) => aggregate.record_review_verdict(
                &intent,
                request.stage(),
                policy.as_ref(),
                request.reviewer(),
                request.iteration(),
                verdict,
                occurred_at,
            ),
        };
        let event = refused.map_err(|error| ReviewLogError::Command {
            stage: request.stage().clone(),
            error,
        })?;
        let outcome = match request.kind() {
            ReviewLogKind::Request { retry_pending } => ReviewLogOutcome::Requested {
                retry: retry_pending,
            },
            ReviewLogKind::Verdict(_) => ReviewLogOutcome::Completed,
        };
        match self
            .intent_execution_repository
            .store(&event, &aggregate)
            .await
        {
            Ok(()) => Ok(AttemptOutcome::Settled(outcome)),
            Err(conflict @ RepositoryError::Conflict { .. }) => {
                Ok(AttemptOutcome::Conflicted(conflict))
            }
            Err(other) => Err(ReviewLogError::Repository(other)),
        }
    }

    /// 対象ステージのレビュー方針を定義から解決する。
    async fn review_policy(
        &self,
        intent: &Intent,
        request: &ReviewLogRequest,
    ) -> Result<Option<ReviewPolicy>, ReviewLogError> {
        let definition = self
            .workflow_definition_repository
            .find_for_intent(intent)
            .await?;
        intent
            .resolve_review_policy(&definition, request.stage())
            .map_err(|error| match error {
                IntentReviewError::UnknownStage => {
                    ReviewLogError::UnknownStage(request.stage().clone())
                }
                IntentReviewError::InvalidOverride(raw) => {
                    ReviewLogError::CorruptReviewOverride(raw)
                }
                error @ IntentReviewError::DefinitionMismatch => {
                    ReviewLogError::ReviewPolicy(error)
                }
            })
    }

    /// 注入された実行ポートの実装（テストが**効果**を観測するための継ぎ目）。
    #[cfg(test)]
    pub(crate) const fn intent_execution_repository(&self) -> &E {
        &self.intent_execution_repository
    }
}

#[cfg(test)]
mod tests {
    // panic! は「想定した変種でなければ即失敗」という検証用途で使う。
    #![allow(clippy::panic)]

    use super::super::port::RepositoryError;
    use super::super::review_log_error::ReviewLogError;
    use super::super::review_log_kind::ReviewLogKind;
    use super::super::review_log_outcome::ReviewLogOutcome;
    use super::super::review_log_request::ReviewLogRequest;
    use super::super::test_support::{
        InMemoryIntentExecutionRepository, InMemoryIntentRepository,
        InMemoryWorkflowDefinitionRepository, absent_execution, at, definition,
        definition_with_reviewer, execution_id, genesis, genesis_with_review, slug,
    };
    use super::RecordReviewUseCase;
    use core_command_domain::orchestration::{
        CommandError, Intent, IntentExecution, IntentExecutionEvent, ReviewVerdict,
    };
    use core_command_domain::workflow_definition::{
        ReviewCapValue, ReviewClass, WorkflowDefinition,
    };

    /// フィクスチャのレビュアー名。
    const REVIEWER: &str = "aidlc-quality-agent";

    struct Subject {
        use_case: RecordReviewUseCase<
            InMemoryIntentExecutionRepository,
            InMemoryIntentRepository,
            InMemoryWorkflowDefinitionRepository,
        >,
    }

    impl Subject {
        async fn execute(
            &mut self,
            request: &ReviewLogRequest,
        ) -> Result<ReviewLogOutcome, ReviewLogError> {
            self.use_case.execute(&execution_id(), request, at()).await
        }

        const fn intent_execution_repository(&self) -> &InMemoryIntentExecutionRepository {
            self.use_case.intent_execution_repository()
        }
    }

    /// 索引 1 にレビュアーを宣言した定義（クラスは引数で決める）。
    fn reviewed(class: Option<ReviewClass>, cap: Option<ReviewCapValue>) -> WorkflowDefinition {
        definition_with_reviewer(3, 1, REVIEWER, class, None, cap)
    }

    fn use_case(definition: WorkflowDefinition) -> Subject {
        let (intent, aggregate, _) = genesis(3);
        use_case_holding((intent, aggregate), 1, definition)
    }

    fn use_case_holding(
        pair: (Intent, IntentExecution),
        version: usize,
        definition: WorkflowDefinition,
    ) -> Subject {
        let (intent, aggregate) = pair;
        Subject {
            use_case: RecordReviewUseCase::new(
                InMemoryIntentExecutionRepository::holding(aggregate, version),
                InMemoryIntentRepository::holding(intent),
                InMemoryWorkflowDefinitionRepository::holding(definition),
            ),
        }
    }

    fn request(iteration: u32) -> ReviewLogRequest {
        ReviewLogRequest::new(
            slug(1),
            REVIEWER,
            iteration,
            ReviewLogKind::Request {
                retry_pending: false,
            },
        )
    }

    fn retry(iteration: u32) -> ReviewLogRequest {
        ReviewLogRequest::new(
            slug(1),
            REVIEWER,
            iteration,
            ReviewLogKind::Request {
                retry_pending: true,
            },
        )
    }

    fn verdict(iteration: u32, verdict: ReviewVerdict) -> ReviewLogRequest {
        ReviewLogRequest::new(
            slug(1),
            REVIEWER,
            iteration,
            ReviewLogKind::Verdict(verdict),
        )
    }

    /// 定型の 3 手 — 再構成 → 集約コマンド → `store`。
    #[tokio::test]
    async fn a_request_commits_a_review_requested_event() {
        let mut subject = use_case(reviewed(None, None));

        let outcome = subject.execute(&request(1)).await.expect("1 回目は通る");

        assert_eq!(outcome, ReviewLogOutcome::Requested { retry: false });
        let committed = subject.intent_execution_repository().committed();
        assert_eq!(committed.len(), 1);
        assert!(matches!(
            committed.first(),
            Some(IntentExecutionEvent::ReviewRequested(requested))
                if requested.iteration() == 1 && !requested.is_retry()
        ));
    }

    /// 判定は依頼と対になったときだけ通り、`Completed` を返す。
    #[tokio::test]
    async fn a_verdict_commits_a_review_completed_event_after_its_request() {
        let mut subject = use_case(reviewed(None, None));
        subject.execute(&request(1)).await.expect("依頼は通る");

        let outcome = subject
            .execute(&verdict(1, ReviewVerdict::Ready))
            .await
            .expect("対になる判定は通る");

        assert_eq!(outcome, ReviewLogOutcome::Completed);
        assert!(matches!(
            subject.intent_execution_repository().committed().last(),
            Some(IntentExecutionEvent::ReviewCompleted(completed))
                if completed.verdict() == ReviewVerdict::Ready
        ));
    }

    /// 呼び直しは `retry: true` を返す（行は出るが依頼には数えない）。
    #[tokio::test]
    async fn a_retry_reports_that_it_was_a_retry() {
        let mut subject = use_case(reviewed(None, None));
        subject.execute(&request(1)).await.expect("依頼は通る");

        let outcome = subject.execute(&retry(1)).await.expect("呼び直しは通る");

        assert_eq!(outcome, ReviewLogOutcome::Requested { retry: true });
        // 数え上げは進まないので、2 回目の通常依頼は依然として 2 番である。
        subject.execute(&request(2)).await.expect("2 回目は通る");
    }

    /// 集約の拒否は材料をそのまま運ぶ（文言は出す側が組む）。
    #[tokio::test]
    async fn an_aggregate_refusal_carries_the_stage_and_the_guard() {
        let mut subject = use_case(reviewed(None, None));

        let err = subject
            .execute(&request(2))
            .await
            .expect_err("1 番から始まらない依頼は断られる");

        assert!(
            matches!(
                &err,
                ReviewLogError::Command {
                    stage,
                    error: CommandError::ReviewOutOfSequence { iteration: 2, expected: 1, .. },
                } if stage == &slug(1)
            ),
            "{err:?}"
        );
        assert!(subject.intent_execution_repository().committed().is_empty());
    }

    /// レビュアーを宣言しないステージは、定義の照会が `None` を返して集約が断る。
    #[tokio::test]
    async fn a_stage_without_a_declared_reviewer_is_refused_by_the_aggregate() {
        let mut subject = use_case(definition(3));

        let err = subject
            .execute(&request(1))
            .await
            .expect_err("宣言が無ければ受領証は書けない");

        assert!(
            matches!(
                &err,
                ReviewLogError::Command {
                    error: CommandError::NoDeclaredReviewer(_),
                    ..
                }
            ),
            "{err:?}"
        );
    }

    /// 定義がその slug を知らなければ、ユースケース自身の失敗として断る。
    #[tokio::test]
    async fn a_slug_the_definition_does_not_know_is_refused_before_the_aggregate() {
        // 定義は 1 段だけ — 計画の索引 1 を知らない。
        let mut subject = use_case(definition(1));

        let err = subject
            .execute(&request(1))
            .await
            .expect_err("定義に無い slug は引けない");

        assert!(
            matches!(&err, ReviewLogError::UnknownStage(stage) if stage == &slug(1)),
            "{err:?}"
        );
    }

    /// 実効クラスは scope の上限で下がる — `none` に落ちれば予算 0 で依頼が断られる。
    #[tokio::test]
    async fn a_scope_cap_that_lowers_the_class_to_none_refuses_the_request() {
        let mut subject = use_case(reviewed(None, Some(ReviewCapValue::None)));

        let err = subject
            .execute(&request(1))
            .await
            .expect_err("実効 none は依頼を受け付けない");

        assert!(
            matches!(
                &err,
                ReviewLogError::Command {
                    error: CommandError::ReviewBudgetExceeded { budget: 0, .. },
                    ..
                }
            ),
            "{err:?}"
        );
    }

    /// advisory は 1 パスで予算を使い切る。
    #[tokio::test]
    async fn an_advisory_class_spends_its_budget_in_one_pass() {
        let mut subject = use_case(reviewed(Some(ReviewClass::Advisory), None));
        subject.execute(&request(1)).await.expect("1 パスは通る");

        let err = subject
            .execute(&request(2))
            .await
            .expect_err("advisory に 2 回目は無い");

        assert!(
            matches!(
                &err,
                ReviewLogError::Command {
                    error: CommandError::ReviewBudgetExceeded { budget: 1, .. },
                    ..
                }
            ),
            "{err:?}"
        );
    }

    /// 楽観競合は 1 回だけ再試行する。
    #[tokio::test]
    async fn a_first_conflict_is_retried_once_from_the_rehydration() {
        let (intent, aggregate, _) = genesis(3);
        let mut subject = Subject {
            use_case: RecordReviewUseCase::new(
                InMemoryIntentExecutionRepository::holding_behind_concurrent_writes(
                    aggregate, 7, 1,
                ),
                InMemoryIntentRepository::holding(intent),
                InMemoryWorkflowDefinitionRepository::holding(reviewed(None, None)),
            ),
        };

        subject
            .execute(&request(1))
            .await
            .expect("1 回だけ再試行すれば通る");

        assert_eq!(
            subject.intent_execution_repository().store_attempts(),
            2,
            "再試行は 1 回だけ"
        );
    }

    /// 2 回目も競合したらそのまま伝播する。
    #[tokio::test]
    async fn a_second_conflict_is_propagated_without_a_further_retry() {
        let (intent, aggregate, _) = genesis(3);
        let mut subject = Subject {
            use_case: RecordReviewUseCase::new(
                InMemoryIntentExecutionRepository::holding_behind_concurrent_writes(
                    aggregate, 7, 2,
                ),
                InMemoryIntentRepository::holding(intent),
                InMemoryWorkflowDefinitionRepository::holding(reviewed(None, None)),
            ),
        };

        let err = subject
            .execute(&request(1))
            .await
            .expect_err("2 回目の競合は伝播する");

        assert!(
            matches!(
                &err,
                ReviewLogError::Repository(RepositoryError::Conflict { .. })
            ),
            "{err:?}"
        );
        assert_eq!(subject.intent_execution_repository().store_attempts(), 2);
    }

    /// `--review` override は実効クラスを下げる（`none` に落ちれば依頼が断られる）。
    #[tokio::test]
    async fn a_review_override_lowers_the_effective_class() {
        let (intent, aggregate, _) = genesis_with_review(3, "none");
        let mut subject = use_case_holding((intent, aggregate), 1, reviewed(None, None));

        let err = subject
            .execute(&request(1))
            .await
            .expect_err("override で none に落ちれば依頼できない");

        assert!(
            matches!(
                &err,
                ReviewLogError::Command {
                    error: CommandError::ReviewBudgetExceeded { budget: 0, .. },
                    ..
                }
            ),
            "{err:?}"
        );
    }

    /// 壊れた `--review` の値は既定へ落とさず surface する。
    #[tokio::test]
    async fn a_corrupt_review_override_is_surfaced_rather_than_defaulted() {
        let (intent, aggregate, _) = genesis_with_review(3, "Adversarial");
        let mut subject = use_case_holding((intent, aggregate), 1, reviewed(None, None));

        let err = subject
            .execute(&request(1))
            .await
            .expect_err("閉集合の外は既定へ落とさない");

        assert!(
            matches!(&err, ReviewLogError::CorruptReviewOverride(raw) if raw == "Adversarial"),
            "{err:?}"
        );
    }

    /// ストアに無い集約は再構成できない。
    #[tokio::test]
    async fn a_missing_aggregate_is_reported_as_not_found() {
        let (intent, _, _) = genesis(3);
        let mut use_case = RecordReviewUseCase::new(
            InMemoryIntentExecutionRepository::empty(),
            InMemoryIntentRepository::holding(intent),
            InMemoryWorkflowDefinitionRepository::holding(reviewed(None, None)),
        );

        let err = use_case
            .execute(&absent_execution(), &request(1), at())
            .await
            .expect_err("ストアに無い集約は再構成できない");

        assert!(
            matches!(
                &err,
                ReviewLogError::Repository(RepositoryError::NotFound { id }) if *id == absent_execution()
            ),
            "{err:?}"
        );
    }

    /// 定義ポートの失敗はそのまま伝播する。
    #[tokio::test]
    async fn a_definition_port_failure_is_propagated_verbatim() {
        let (intent, aggregate, _) = genesis(3);
        let mut subject = Subject {
            use_case: RecordReviewUseCase::new(
                InMemoryIntentExecutionRepository::holding(aggregate, 1),
                InMemoryIntentRepository::holding(intent),
                InMemoryWorkflowDefinitionRepository::empty(),
            ),
        };

        let err = subject
            .execute(&request(1))
            .await
            .expect_err("定義が無ければ方針が解決できない");

        assert!(
            matches!(
                &err,
                ReviewLogError::DefinitionRepository(RepositoryError::NotFound { .. })
            ),
            "{err:?}"
        );
    }
}
