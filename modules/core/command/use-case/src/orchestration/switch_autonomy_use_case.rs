//! `SwitchAutonomyUseCase` — 自律モードの切替を記録する（`aidlc-bolt set-autonomy`、b50 / I11）。

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::IntentExecutionId;

use super::autonomy_switch_request::AutonomySwitchRequest;
use super::port::IntentExecutionRepository;
use super::port::IntentRepository;
use super::port::RepositoryError;
use super::switch_autonomy_error::SwitchAutonomyError;

/// 自律モードの切替を 1 件記録する（`aidlc-bolt set-autonomy`）。
///
/// 定型は [`super::PromotePracticesUseCase`] と同じ 3 手である: **`find_by_id` で集約を再構成 →
/// 集約コマンドで判断 → `store` で保存**。定義は引かない — 切替は計画上のどのステージにも
/// 紐づかないからである（設計 §4）。
///
/// # ここに無いもの
///
/// - **業務判断**。「昇格を受理してよいか」（human presence ガード、I11）は
///   `IntentExecution::switch_autonomy` が持つ。材料の 2 分類（集約が状態として持つ直近の
///   ゲート解決時刻と、外部の入力である `HumanTurns`）もそこで合流する。
/// - **材料の読取**。監査台帳を読んで [`HumanTurns`] を組むのも、env
///   `AIDLC_SKIP_HUMAN_PRESENCE_GUARD` を見るのも合成ルートである。
/// - **文言**。`Refusing to switch Construction to autonomous: …` の逐語は合成ルートが組む。
/// - **リードモデルの更新**。`catch_up` を起動するのは合成ルートである。
///
/// # 成功は `Ok(())` である（CQS）
///
/// stdout の 1 行に要る材料（切替先のモード）は**合成ルートが持っている** — 自分が組んだ
/// [`AutonomySwitchRequest`] の値だからである（`coding-rules/command-query-separation.md`）。
///
/// [`HumanTurns`]: core_command_domain::workspace::HumanTurns
#[derive(Debug)]
pub struct SwitchAutonomyUseCase<E: IntentExecutionRepository, I: IntentRepository> {
    intent_execution_repository: E,
    intent_repository: I,
}

/// [`SwitchAutonomyUseCase::attempt`] 1 回分の結末。
#[derive(Debug)]
enum AttemptOutcome {
    /// 決着した — 切替の行をコミットした。
    Settled,
    /// 楽観 version が競合した（2 回目も競合したらこれを伝播する）。
    Conflicted(RepositoryError<IntentExecutionId>),
}

impl<E: IntentExecutionRepository, I: IntentRepository> SwitchAutonomyUseCase<E, I> {
    /// ポートの実装を 2 つ注入する（**この型の唯一の構築経路**）。
    #[must_use]
    pub const fn new(
        intent_execution_repository: E,
        intent_repository: I,
    ) -> SwitchAutonomyUseCase<E, I> {
        SwitchAutonomyUseCase {
            intent_execution_repository,
            intent_repository,
        }
    }

    /// 切替を 1 件記録する。
    ///
    /// `occurred_at` は呼出側が持つ時計の読みである — 集約は時計を持たない（NFR3.1）。
    ///
    /// # Errors
    ///
    /// 実行・intent の再構成や永続化の失敗、集約による拒否（`Command`）を返す。
    pub async fn execute(
        &mut self,
        execution_id: &IntentExecutionId,
        request: &AutonomySwitchRequest,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), SwitchAutonomyError> {
        match self.attempt(execution_id, request, occurred_at).await? {
            AttemptOutcome::Settled => Ok(()),
            AttemptOutcome::Conflicted(_) => {
                match self.attempt(execution_id, request, occurred_at).await? {
                    AttemptOutcome::Settled => Ok(()),
                    AttemptOutcome::Conflicted(conflict) => {
                        Err(SwitchAutonomyError::Repository(conflict))
                    }
                }
            }
        }
    }

    /// 再構成からコミットまでの 1 回分。競合したときはこれをもう 1 度だけ通す。
    async fn attempt(
        &mut self,
        execution_id: &IntentExecutionId,
        request: &AutonomySwitchRequest,
        occurred_at: DateTime<Utc>,
    ) -> Result<AttemptOutcome, SwitchAutonomyError> {
        let mut aggregate = self
            .intent_execution_repository
            .find_by_id(execution_id)
            .await?;
        let intent = self
            .intent_repository
            .find_by_id(aggregate.intent_id())
            .await?;
        let event = aggregate
            .switch_autonomy(
                &intent,
                request.mode(),
                request.turns(),
                request.is_human_presence_guard(),
                occurred_at,
            )
            .map_err(SwitchAutonomyError::Command)?;
        match self
            .intent_execution_repository
            .store(&event, &aggregate)
            .await
        {
            Ok(()) => Ok(AttemptOutcome::Settled),
            Err(conflict @ RepositoryError::Conflict { .. }) => {
                Ok(AttemptOutcome::Conflicted(conflict))
            }
            Err(other) => Err(SwitchAutonomyError::Repository(other)),
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

    use super::super::autonomy_switch_request::AutonomySwitchRequest;
    use super::super::port::RepositoryError;
    use super::super::switch_autonomy_error::SwitchAutonomyError;
    use super::super::test_support::{
        InMemoryIntentExecutionRepository, InMemoryIntentRepository, absent_execution, at,
        execution_id, genesis,
    };
    use super::SwitchAutonomyUseCase;
    use core_command_domain::orchestration::{AutonomyMode, CommandError, IntentExecutionEvent};
    use core_command_domain::workspace::HumanTurns;

    /// ガードを外した昇格（材料の読取はユースケースの外なので既定の台帳でよい）。
    fn request() -> AutonomySwitchRequest {
        AutonomySwitchRequest::new(AutonomyMode::Autonomous, HumanTurns::default(), false)
    }

    /// ガードを効かせた昇格（人間の turn が 1 つも無い台帳）。
    fn guarded_request() -> AutonomySwitchRequest {
        let turns = HumanTurns::find_in(
            "\n## H\n**Timestamp**: 2026-08-23T00:00:00Z\n**Event**: STAGE_STARTED\n",
        );
        AutonomySwitchRequest::new(AutonomyMode::Autonomous, turns, true)
    }

    fn subject()
    -> SwitchAutonomyUseCase<InMemoryIntentExecutionRepository, InMemoryIntentRepository> {
        let (intent, execution, _) = genesis(3);
        SwitchAutonomyUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution, 1),
            InMemoryIntentRepository::holding(intent),
        )
    }

    /// 3 手が通り、イベント 1 件がストアへ着く。
    #[tokio::test]
    async fn the_switch_commits_one_event() {
        let mut use_case = subject();
        use_case
            .execute(&execution_id(), &request(), at())
            .await
            .expect("切替は通る");
        let committed = use_case.intent_execution_repository().committed();
        assert_eq!(committed.len(), 1);
        let Some(IntentExecutionEvent::AutonomyModeSet(set)) = committed.first() else {
            panic!("AutonomyModeSet を期待した: {committed:?}");
        };
        assert_eq!(set.mode(), AutonomyMode::Autonomous);
    }

    /// 集約の拒否はそのまま伝播する（昇格 + ガード + 人間の turn 無し）。
    #[tokio::test]
    async fn a_guarded_escalation_without_a_human_turn_propagates_the_guard() {
        let mut use_case = subject();
        let error = use_case
            .execute(&execution_id(), &guarded_request(), at())
            .await
            .expect_err("人間の turn が無いので断られる");
        assert!(
            matches!(
                &error,
                SwitchAutonomyError::Command(CommandError::HumanPresenceRequired)
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
                SwitchAutonomyError::Repository(RepositoryError::NotFound { .. })
            ),
            "{error:?}"
        );
    }

    /// intent が引けなければ intent ポートの失敗を運ぶ。
    #[tokio::test]
    async fn an_absent_intent_propagates_the_intent_port_failure() {
        let (_, execution, _) = genesis(3);
        let mut use_case = SwitchAutonomyUseCase::new(
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
                SwitchAutonomyError::IntentRepository(RepositoryError::NotFound { .. })
            ),
            "{error:?}"
        );
    }

    /// 楽観競合は 1 回だけ再試行する（2 回目で通る）。
    #[tokio::test]
    async fn a_conflict_is_retried_once() {
        let (intent, execution, _) = genesis(3);
        let mut use_case = SwitchAutonomyUseCase::new(
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
        let (intent, execution, _) = genesis(3);
        let mut use_case = SwitchAutonomyUseCase::new(
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
                SwitchAutonomyError::Repository(RepositoryError::Conflict { .. })
            ),
            "{error:?}"
        );
    }
}
