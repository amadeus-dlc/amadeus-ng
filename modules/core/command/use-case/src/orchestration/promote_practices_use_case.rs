//! `PromotePracticesUseCase` — 承認された実践の昇格を記録する
//! （`aidlc-state practices-promote`、b49 / B10）。

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::IntentExecutionId;

use super::port::IntentExecutionRepository;
use super::port::IntentRepository;
use super::port::RepositoryError;
use super::practices_promotion_request::PracticesPromotionRequest;
use super::promote_practices_error::PromotePracticesError;

/// 昇格の事実を 1 件記録する（`aidlc-state practices-promote`）。
///
/// 定型は [`super::RecordReviewUseCase`] と同じ 3 手である: **`find_by_id` で集約を再構成 →
/// 集約コマンドで判断 → `store` で保存**。違うのは**定義を引かない**という点だけである —
/// practices ステージは実行の計画に載っている slug で見つかるので、定義側の静的材料が要らない
/// （設計 §4）。
///
/// # ここに無いもの
///
/// - **業務判断**。「受け取ってよいか」も「どのステージの受領証が立つか」も
///   `IntentExecution::affirm_practices` が持つ。
/// - **昇格内容の計算**。ドラフト 2 本と正本 2 本から節と規則行を決めるのは
///   `PracticesPromotion::plan`（純関数）であり、それを呼ぶのは合成ルートである。
/// - **文言**。`practices-promote failed: …` の逐語は合成ルートが組む。
/// - **リードモデルの更新**。`catch_up` を起動するのは合成ルートである。
///
/// # 成功は `Ok(())` である（CQS）
///
/// stdout の 1 行に要る材料（発生時刻・件数・書込先）は**合成ルートが全部持っている** —
/// 発生時刻は自分が渡した `occurred_at`、件数は自分が組んだ [`PracticesPromotionRequest`]、
/// パスは自分が解決した memory 層である。したがって戻り値で運ぶものが無い
/// （`coding-rules/command-query-separation.md`）。
#[derive(Debug)]
pub struct PromotePracticesUseCase<E: IntentExecutionRepository, I: IntentRepository> {
    intent_execution_repository: E,
    intent_repository: I,
}

/// [`PromotePracticesUseCase::attempt`] 1 回分の結末。
#[derive(Debug)]
enum AttemptOutcome {
    /// 決着した — 昇格の行をコミットした。
    Settled,
    /// 楽観 version が競合した（2 回目も競合したらこれを伝播する）。
    Conflicted(RepositoryError<IntentExecutionId>),
}

impl<E: IntentExecutionRepository, I: IntentRepository> PromotePracticesUseCase<E, I> {
    /// ポートの実装を 2 つ注入する（**この型の唯一の構築経路**）。
    #[must_use]
    pub const fn new(
        intent_execution_repository: E,
        intent_repository: I,
    ) -> PromotePracticesUseCase<E, I> {
        PromotePracticesUseCase {
            intent_execution_repository,
            intent_repository,
        }
    }

    /// 昇格を 1 件記録する。
    ///
    /// `occurred_at` は呼出側が持つ時計の読みである — 集約は時計を持たない（NFR3.1）。
    ///
    /// # Errors
    ///
    /// 実行・intent の再構成や永続化の失敗、集約による拒否（`Command`）を返す。
    pub async fn execute(
        &mut self,
        execution_id: &IntentExecutionId,
        request: &PracticesPromotionRequest,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), PromotePracticesError> {
        match self.attempt(execution_id, request, occurred_at).await? {
            AttemptOutcome::Settled => Ok(()),
            AttemptOutcome::Conflicted(_) => {
                match self.attempt(execution_id, request, occurred_at).await? {
                    AttemptOutcome::Settled => Ok(()),
                    AttemptOutcome::Conflicted(conflict) => {
                        Err(PromotePracticesError::Repository(conflict))
                    }
                }
            }
        }
    }

    /// 再構成からコミットまでの 1 回分。競合したときはこれをもう 1 度だけ通す。
    async fn attempt(
        &mut self,
        execution_id: &IntentExecutionId,
        request: &PracticesPromotionRequest,
        occurred_at: DateTime<Utc>,
    ) -> Result<AttemptOutcome, PromotePracticesError> {
        let mut aggregate = self
            .intent_execution_repository
            .find_by_id(execution_id)
            .await?;
        let intent = self
            .intent_repository
            .find_for_execution(&aggregate)
            .await?;
        let event = aggregate
            .affirm_practices(
                &intent,
                request.promotion(),
                request.affirming_user(),
                occurred_at,
            )
            .map_err(PromotePracticesError::Command)?;
        match self
            .intent_execution_repository
            .store(&event, &aggregate)
            .await
        {
            Ok(()) => Ok(AttemptOutcome::Settled),
            Err(conflict @ RepositoryError::Conflict { .. }) => {
                Ok(AttemptOutcome::Conflicted(conflict))
            }
            Err(other) => Err(PromotePracticesError::Repository(other)),
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
    // panic! は「想定した変種でなければ即失敗」という検証用途で使う。
    #![allow(clippy::panic)]

    use super::super::port::RepositoryError;
    use super::super::practices_promotion_request::PracticesPromotionRequest;
    use super::super::promote_practices_error::PromotePracticesError;
    use super::super::test_support::{
        InMemoryIntentExecutionRepository, InMemoryIntentRepository, absent_execution, at,
        execution_id, genesis, genesis_with_practices,
    };
    use super::PromotePracticesUseCase;
    use core_command_domain::orchestration::{CommandError, IntentExecutionEvent};
    use core_command_domain::workspace::{PracticesPromotion, PromotedSection};

    fn request() -> PracticesPromotionRequest {
        PracticesPromotionRequest::new(promotion(), "owner")
    }

    /// 合成の昇格（節 1 つと規則 1 本 — イベントに材料が載ることを見る）。
    fn promotion() -> PracticesPromotion {
        PracticesPromotion::plan(
            "## Way of Working\ntrunk-based.\n",
            "## Mandated\nALWAYS review.\n",
            "# Team\n\n## Way of Working\nold.\n",
            "# Project\n\n## Mandated\n\n## Forbidden\n",
            chrono::NaiveDate::from_ymd_opt(2026, 9, 5).expect("固定の日付"),
        )
        .expect("フィクスチャの昇格は組める")
    }

    /// 索引 1 が practices-discovery の計画で、ストアに据えたユースケース。
    fn subject()
    -> PromotePracticesUseCase<InMemoryIntentExecutionRepository, InMemoryIntentRepository> {
        let (intent, execution, _) = genesis_with_practices(3);
        PromotePracticesUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution, 1),
            InMemoryIntentRepository::holding(intent),
        )
    }

    /// 3 手が通り、イベント 1 件がストアへ着く。
    #[tokio::test]
    async fn the_promotion_commits_one_event() {
        let mut use_case = subject();
        use_case
            .execute(&execution_id(), &request(), at())
            .await
            .expect("昇格は通る");
        let committed = use_case.intent_execution_repository().committed();
        assert_eq!(committed.len(), 1);
        let Some(IntentExecutionEvent::PracticesAffirmed(affirmed)) = committed.first() else {
            panic!("PracticesAffirmed を期待した: {committed:?}");
        };
        assert_eq!(affirmed.stage().as_str(), "practices-discovery");
        assert_eq!(affirmed.affirming_user(), "owner");
        assert_eq!(
            affirmed.sections().first().map(PromotedSection::heading),
            Some("Way of Working")
        );
        assert_eq!(
            affirmed.mandated(),
            ["ALWAYS review. (affirmed 2026-09-05)".to_string()]
        );
    }

    /// 集約の拒否はそのまま伝播する（計画に practices-discovery が無い形）。
    #[tokio::test]
    async fn a_plan_without_the_stage_propagates_the_guard() {
        let (intent, execution, _) = genesis(3);
        let mut use_case = PromotePracticesUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution, 1),
            InMemoryIntentRepository::holding(intent),
        );
        let error = use_case
            .execute(&execution_id(), &request(), at())
            .await
            .expect_err("計画に無いので断られる");
        assert!(
            matches!(
                &error,
                PromotePracticesError::Command(CommandError::UnknownStage(slug))
                    if slug == "practices-discovery"
            ),
            "{error:?}"
        );
        assert!(
            use_case
                .intent_execution_repository()
                .committed()
                .is_empty()
        );
    }

    /// 実行が居なければポートの失敗をそのまま運ぶ。
    #[tokio::test]
    async fn an_absent_execution_propagates_the_port_failure() {
        let mut use_case = subject();
        let error = use_case
            .execute(&absent_execution(), &request(), at())
            .await
            .expect_err("居ない実行は引けない");
        assert!(
            matches!(
                error,
                PromotePracticesError::Repository(RepositoryError::NotFound { .. })
            ),
            "{error:?}"
        );
    }

    /// intent が引けなければ intent ポートの失敗を運ぶ。
    #[tokio::test]
    async fn an_absent_intent_propagates_the_intent_port_failure() {
        let (_, execution, _) = genesis_with_practices(3);
        let mut use_case = PromotePracticesUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution, 1),
            InMemoryIntentRepository::empty(),
        );
        let error = use_case
            .execute(&execution_id(), &request(), at())
            .await
            .expect_err("intent が引けない");
        assert!(
            matches!(
                error,
                PromotePracticesError::IntentRepository(RepositoryError::NotFound { .. })
            ),
            "{error:?}"
        );
    }

    /// 楽観競合は 1 回だけ再試行する（2 回目で通る）。
    #[tokio::test]
    async fn a_conflict_is_retried_once() {
        let (intent, execution, _) = genesis_with_practices(3);
        let mut use_case = PromotePracticesUseCase::new(
            InMemoryIntentExecutionRepository::holding_behind_concurrent_writes(execution, 1, 1),
            InMemoryIntentRepository::holding(intent),
        );
        use_case
            .execute(&execution_id(), &request(), at())
            .await
            .expect("再試行で通る");
        assert_eq!(use_case.intent_execution_repository().committed().len(), 1);
    }

    /// 2 回目も競合したら伝播する。
    #[tokio::test]
    async fn a_second_conflict_is_propagated() {
        let (intent, execution, _) = genesis_with_practices(3);
        let mut use_case = PromotePracticesUseCase::new(
            InMemoryIntentExecutionRepository::holding_behind_concurrent_writes(execution, 1, 2),
            InMemoryIntentRepository::holding(intent),
        );
        let error = use_case
            .execute(&execution_id(), &request(), at())
            .await
            .expect_err("2 回目も競合する");
        assert!(
            matches!(
                error,
                PromotePracticesError::Repository(RepositoryError::Conflict { .. })
            ),
            "{error:?}"
        );
    }
}
