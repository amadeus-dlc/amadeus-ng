//! `ParkUseCase` — 実行に park マーカーを据える（#74）。

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::IntentExecutionId;

use super::park_error::ParkError;
use super::port::IntentExecutionRepository;
use super::port::IntentRepository;
use super::port::RepositoryError;

/// 実行を現在地で止め、park マーカーを据える（`aidlc-orchestrate park`）。
///
/// 定型は `CommitVerdictUseCase` と同じ 3 手である: **`find_by_id` で集約を再構成 →
/// 集約コマンドで判断 → `store` で保存**。
///
/// # ここに無いもの
///
/// - **業務判断**。park を受理するか、どの順で拒否するか（autonomous → Completed）は
///   すべて `IntentExecution::park` が持つ。park 済みへの park を成功させる**再スタンプ**の
///   意味論も集約側にある（`coding-rules/tell-dont-ask.md` — 判断は状態の所有者へ）。
/// - **文言**。`Refusing to park: ...` のような逐語は合成ルートの `wording` が組む。
/// - **park した位置**。[`ParkUseCase::execute`] は成功しても値を返さない（下記 CQS）。
///   位置は投影後のリードモデル（`read_execution.parked_at_slug`）から読む — upstream の
///   `handlePark` も mutation 後に状態ファイルの `Parked At Stage` を読み直す。
/// - **リードモデルの更新**。`catch_up` を起動するのは合成ルートである。
///
/// # 束縛はスタティック
///
/// `dyn` は使わない（`coding-rules/use-case-rules.md` §2）。結線（実物 / インメモリの選択）は
/// 合成ルートだけが行い、ユースケースはポートの trait しか知らない。
#[derive(Debug)]
pub struct ParkUseCase<E: IntentExecutionRepository, I: IntentRepository> {
    intent_execution_repository: E,
    intent_repository: I,
}

/// [`ParkUseCase::attempt`] 1 回分の結末。
///
/// 楽観 version の競合だけを `Err` から切り出しているのは、**1 回だけ再構成からやり直す**
/// ためである。`CommitVerdictUseCase` と違って再試行の対象を名指しする必要は無い — park は
/// ステージを引数に取らず、常に「そのときのカーソル」に作用するからである。
#[derive(Debug)]
enum AttemptOutcome {
    /// 決着した — マーカーをコミットした。
    Settled,
    /// 楽観 version が競合した（2 回目も競合したらこれを伝播する）。
    Conflicted(RepositoryError<IntentExecutionId>),
}

impl<E: IntentExecutionRepository, I: IntentRepository> ParkUseCase<E, I> {
    /// ポートの実装を 2 つ注入する（**この型の唯一の構築経路**）。
    #[must_use]
    pub const fn new(intent_execution_repository: E, intent_repository: I) -> ParkUseCase<E, I> {
        ParkUseCase {
            intent_execution_repository,
            intent_repository,
        }
    }

    /// 実行を現在地で止める。
    ///
    /// `occurred_at` は呼出側が持つ時計の読みである — 集約は時計を持たない（NFR3.1）。
    ///
    /// # 引数は集約 ID と値だけである
    ///
    /// ユースケースの `execute` に**集約を渡さない** — 渡してよいのは集約 ID と値オブジェクト
    /// だけで、集約は保持するリポジトリから内部で取る
    /// （`coding-rules/use-case-rules.md` §2b）。
    ///
    /// # 戻り値を持たない理由（CQS）
    ///
    /// 状態を変えるので Command であり、CQS が定める Command の形（`&mut self` +
    /// `Result<(), E>`）をそのまま採る（`coding-rules/command-query-separation.md`）。
    /// park した位置は、合成ルートが `catch_up` 後のリードモデルから引く。
    ///
    /// # `Conflict` は 1 回だけ再試行する
    ///
    /// 楽観 version の競合だけがこのユースケースの持つ唯一の再試行政策である
    /// （contract-design Q6 = A）。再試行は**再構成からやり直す** — 古い集約に `store` だけ
    /// 打ち直すのは、読んだ時点の版で書くという楽観ロックの意味そのものを壊す。2 回目も
    /// `Conflict` なら伝播する。
    ///
    /// 再構成し直した集約が別の理由で park を拒む（相手が先に完了させた等）場合も、その拒否を
    /// そのまま伝播する — 握り潰しも言い換えもしない。
    ///
    /// # Errors
    ///
    /// 実行の再構成・永続化の失敗（`Repository`）、計画の取得の失敗（`IntentRepository`）、
    /// 集約による拒否（`Command`）を返す。
    pub async fn execute(
        &mut self,
        execution_id: &IntentExecutionId,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), ParkError> {
        if let AttemptOutcome::Settled = self.attempt(execution_id, occurred_at).await? {
            return Ok(());
        }
        match self.attempt(execution_id, occurred_at).await? {
            AttemptOutcome::Settled => Ok(()),
            AttemptOutcome::Conflicted(conflict) => Err(ParkError::Repository(conflict)),
        }
    }

    /// 再構成からコミットまでの 1 回分。競合したときはこれをもう 1 度だけ通す。
    ///
    /// # Errors
    ///
    /// 競合**以外**の失敗（再構成・計画の取得・集約の拒否）。楽観 version の競合だけは `Err`
    /// ではなく [`AttemptOutcome::Conflicted`] で返す — 呼出側が再試行を決めるためである。
    async fn attempt(
        &mut self,
        execution_id: &IntentExecutionId,
        occurred_at: DateTime<Utc>,
    ) -> Result<AttemptOutcome, ParkError> {
        // 再構成した集約は**ストアが刻んだ版を運んでいる**ので、書込へはそれをそのまま提示
        // する（版は不透明なトークンであり `aggregate.seq_nr()` から導いてはならない）。
        let mut aggregate = self
            .intent_execution_repository
            .find_by_id(execution_id)
            .await?;
        // 計画は保持しているリポジトリから内部で取る。実行は intent を ID で参照するだけ
        // なので（`coding-rules/aggregate-references.md`）、その ID で引く。
        let intent = self
            .intent_repository
            .find_by_id(aggregate.intent_id())
            .await?;
        let event = aggregate.park(&intent, occurred_at)?;
        match self
            .intent_execution_repository
            .store(&event, &aggregate)
            .await
        {
            Ok(()) => Ok(AttemptOutcome::Settled),
            Err(conflict @ RepositoryError::Conflict { .. }) => {
                Ok(AttemptOutcome::Conflicted(conflict))
            }
            Err(other) => Err(ParkError::Repository(other)),
        }
    }

    /// 注入された実行ポートの実装（テストが**効果**を観測するための継ぎ目）。
    #[cfg(test)]
    pub(crate) const fn intent_execution_repository(&self) -> &E {
        &self.intent_execution_repository
    }

    /// 注入された intent ポートの実装（テストが取得回数を観測するための継ぎ目）。
    #[cfg(test)]
    pub(crate) const fn intent_repository(&self) -> &I {
        &self.intent_repository
    }
}

#[cfg(test)]
mod tests {
    // panic! は「想定した変種でなければ即失敗」という検証用途で使っており、テスト失敗の
    // シグナルとして妥当なので許容する（`CommitVerdictUseCase` のテストと同じ作法）。
    #![allow(clippy::panic)]

    use super::super::park_error::ParkError;
    use super::super::park_use_case::ParkUseCase;
    use super::super::port::RepositoryError;
    use super::super::test_support::{
        InMemoryIntentExecutionRepository, InMemoryIntentRepository, absent_execution, at,
        execution_id, genesis, slug,
    };
    use core_command_domain::orchestration::{
        AutonomyMode, CommandError, Intent, IntentExecution, IntentExecutionEvent, Status,
    };

    /// テストの主体 — 2 本のポートを注入したユースケース。
    struct Subject {
        use_case: ParkUseCase<InMemoryIntentExecutionRepository, InMemoryIntentRepository>,
    }

    impl Subject {
        async fn execute(&mut self) -> Result<(), ParkError> {
            self.use_case.execute(&execution_id(), at()).await
        }

        const fn intent_execution_repository(&self) -> &InMemoryIntentExecutionRepository {
            self.use_case.intent_execution_repository()
        }

        const fn intent_repository(&self) -> &InMemoryIntentRepository {
            self.use_case.intent_repository()
        }
    }

    fn use_case(pair: (Intent, IntentExecution), version: usize) -> Subject {
        let (intent, aggregate) = pair;
        Subject {
            use_case: ParkUseCase::new(
                InMemoryIntentExecutionRepository::holding(aggregate, version),
                InMemoryIntentRepository::holding(intent),
            ),
        }
    }

    /// カーソルが最初のゲート付きステージに立っている実行。
    fn at_the_first_gate(stage_count: usize) -> (Intent, IntentExecution) {
        let (intent, aggregate, _) = genesis(stage_count);
        (intent, aggregate)
    }

    /// **効果**の観測 — ストアが受理した唯一のイベントを取り出す。
    fn only_committed(
        intent_execution_repository: &InMemoryIntentExecutionRepository,
    ) -> &IntentExecutionEvent {
        let committed = intent_execution_repository.committed();
        assert_eq!(committed.len(), 1, "コミットは 1 件のはず");
        committed.first().expect("1 件ある")
    }

    #[tokio::test]
    async fn parking_commits_the_marker_at_the_cursor() {
        let mut subject = use_case(at_the_first_gate(3), 1);

        subject
            .execute()
            .await
            .expect("Running な実行は park できる");

        let IntentExecutionEvent::Parked(parked) =
            only_committed(subject.intent_execution_repository())
        else {
            panic!("Parked を期待した");
        };
        assert_eq!(parked.stage(), &slug(1), "park の位置はカーソル");
    }

    #[tokio::test]
    async fn the_use_case_fetches_the_intent_itself_from_the_port() {
        let mut subject = use_case(at_the_first_gate(3), 1);
        assert_eq!(subject.intent_repository().lookups(), 0, "呼ぶ前は 0 回");

        subject.execute().await.expect("park は通る");

        assert_eq!(
            subject.intent_repository().lookups(),
            1,
            "1 試行につき 1 回引く"
        );
    }

    #[tokio::test]
    async fn parking_an_already_parked_execution_succeeds_again() {
        // upstream の `handlePark` は park 済みでも成功する（再スタンプ）。
        let (intent, mut aggregate) = at_the_first_gate(3);
        aggregate.park(&intent, at()).expect("最初の park は通る");
        let mut subject = use_case((intent, aggregate), 2);

        subject.execute().await.expect("park 済みでももう一度通る");

        assert!(matches!(
            only_committed(subject.intent_execution_repository()),
            IntentExecutionEvent::Parked(_)
        ));
    }

    #[tokio::test]
    async fn an_autonomous_run_is_refused_by_the_aggregate() {
        let (intent, mut aggregate) = at_the_first_gate(3);
        aggregate
            .switch_autonomy(&intent, AutonomyMode::Autonomous, at())
            .expect("モード切替は通る");
        let mut subject = use_case((intent, aggregate), 2);

        let err = subject
            .execute()
            .await
            .expect_err("autonomous は park しない");

        assert!(matches!(
            err,
            ParkError::Command(CommandError::RefusedUnderAutonomy)
        ));
        assert!(subject.intent_execution_repository().committed().is_empty());
    }

    #[tokio::test]
    async fn a_completed_workflow_is_refused_by_the_aggregate() {
        let (intent, mut aggregate) = at_the_first_gate(2);
        aggregate
            .approve_gate(&intent, None, at())
            .expect("最後のゲートは承認できる");
        assert_eq!(aggregate.status(), Status::Completed);
        let mut subject = use_case((intent, aggregate), 2);

        let err = subject.execute().await.expect_err("完了済みは park しない");

        assert!(matches!(err, ParkError::Command(CommandError::NotRunning)));
        assert!(subject.intent_execution_repository().committed().is_empty());
    }

    #[tokio::test]
    async fn a_missing_aggregate_is_reported_as_not_found() {
        let (intent, _, _) = genesis(3);
        let mut use_case = ParkUseCase::new(
            InMemoryIntentExecutionRepository::empty(),
            InMemoryIntentRepository::holding(intent),
        );

        let err = use_case
            .execute(&absent_execution(), at())
            .await
            .expect_err("ストアに無い集約は再構成できない");

        assert!(matches!(
            err,
            ParkError::Repository(RepositoryError::NotFound { id }) if id == absent_execution()
        ));
    }

    #[tokio::test]
    async fn a_missing_intent_is_propagated_from_its_own_port() {
        let (intent, aggregate) = at_the_first_gate(3);
        let mut use_case = ParkUseCase::new(
            InMemoryIntentExecutionRepository::holding(aggregate, 7),
            InMemoryIntentRepository::empty(),
        );

        let err = use_case
            .execute(&execution_id(), at())
            .await
            .expect_err("計画が引けなければ park できない");

        assert!(matches!(
            err,
            ParkError::IntentRepository(RepositoryError::NotFound { id }) if id == *intent.id()
        ));
    }

    #[tokio::test]
    async fn a_first_conflict_is_retried_once_from_the_rehydration() {
        let (intent, aggregate) = at_the_first_gate(3);
        let mut subject = Subject {
            use_case: ParkUseCase::new(
                InMemoryIntentExecutionRepository::holding_behind_concurrent_writes(
                    aggregate, 7, 1,
                ),
                InMemoryIntentRepository::holding(intent),
            ),
        };

        subject.execute().await.expect("1 回だけ再試行すれば通る");

        assert!(matches!(
            only_committed(subject.intent_execution_repository()),
            IntentExecutionEvent::Parked(_)
        ));
        assert_eq!(
            subject.intent_execution_repository().store_attempts(),
            2,
            "再試行は 1 回だけ"
        );
        assert_eq!(
            subject
                .intent_execution_repository()
                .version_of(&execution_id()),
            Some(9),
            "版 8 を提示して書けたので、ストアは 9 を採番した"
        );
        assert_eq!(
            subject.intent_repository().lookups(),
            2,
            "再試行は attempt 全体をやり直すので計画も引き直す"
        );
    }

    #[tokio::test]
    async fn a_second_conflict_is_propagated_without_a_further_retry() {
        let (intent, aggregate) = at_the_first_gate(3);
        let mut subject = Subject {
            use_case: ParkUseCase::new(
                InMemoryIntentExecutionRepository::holding_behind_concurrent_writes(
                    aggregate, 7, 2,
                ),
                InMemoryIntentRepository::holding(intent),
            ),
        };

        let err = subject
            .execute()
            .await
            .expect_err("2 回目も競合したら伝播する");

        assert!(matches!(
            err,
            ParkError::Repository(RepositoryError::Conflict {
                expected: 8,
                actual: 9,
            })
        ));
        assert_eq!(
            subject.intent_execution_repository().store_attempts(),
            2,
            "3 回目は打たない"
        );
        assert!(subject.intent_execution_repository().committed().is_empty());
    }
}
