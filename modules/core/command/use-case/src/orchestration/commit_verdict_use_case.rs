//! `CommitVerdictUseCase` — 報告された結末を 1 つの遷移としてコミットする（FR2.1）。

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    Intent, IntentExecution, IntentExecutionEvent, IntentExecutionId, ReportDecision, ReportNoOp,
    ReportRequest, TransitionStep,
};
use core_command_domain::workflow_definition::{ReviewCapValue, ReviewPolicy, StageSlug};

use super::commit_error::CommitError;
use super::commit_outcome::CommitOutcome;
use super::port::IntentExecutionRepository;
use super::port::IntentRepository;
use super::port::RepositoryError;
use super::port::WorkflowDefinitionRepository;

/// コンダクタが報告した結末（[`ReportRequest`]）を 1 つの遷移としてコミットする。
///
/// 定型は 3 手である: **`find_by_id` で集約を再構成 → 集約のクエリ `report_dispatch` に
/// 何を打つか訊く → 決まった集約コマンドを打って `store` で保存**。
///
/// # 名前について
///
/// upstream の CLI 動詞は `report` だが、その綴りをそのまま型名にすると「レポート（帳票）を
/// 作る／読むユースケース」と読める（オーナー裁定 2026-08-29 — 実際に誤読された）。型名は
/// **更新の意図を先頭に置く**ものへ改めた。CLI 動詞と型の対応は U7 の ROUTES 表が持つので、
/// 型名が upstream の綴りに縛られる必要はない。
///
/// # ここに無いもの
///
/// - **業務判断**。13 段ガードのうち状態で決まる段（`skipped` の受理 5 条件・gate 前提・
///   human presence・forward 表・stale 再報告）はすべて集約のクエリ
///   [`IntentExecution::report_dispatch`] が持つ。ここにあるのは「決まったとおりに打つ」
///   フロー制御だけである（`coding-rules/tell-dont-ask.md` — 判断は状態の所有者へ）。
/// - **文言**。「Committed approve for "..."」のような逐語は合成ルート（U7）の Presenter が
///   組む。ここが返すのは材料（[`CommitOutcome`]）だけである。
/// - **リードモデルの更新**。`aidlc-state.md` と監査シャードを最新化する `ReadModelUpdater` を
///   起動するのは合成ルート（U7）である。コマンド側のユースケースはクエリ側を知らない
///   （`coding-rules/cqrs-boundaries.md` — 境界はクレート分離で物理強制されている）。
/// - **`resume` のルーティング**。遷移をコミットしないので合成ルートが手前で分岐する
///   （`coding-rules/use-case-rules.md` §3）。集約まで届いた場合は
///   `ReportRefusal::RoutedVerdict` になる。
///
/// # 束縛はスタティック
///
/// `dyn` は使わない（`coding-rules/use-case-rules.md` §2）。結線（実物 / インメモリの選択）は
/// 合成ルートだけが行い、ユースケースはポートの trait しか知らない。
#[derive(Debug)]
pub struct CommitVerdictUseCase<
    E: IntentExecutionRepository,
    I: IntentRepository,
    D: WorkflowDefinitionRepository,
> {
    intent_execution_repository: E,
    intent_repository: I,
    workflow_definition_repository: D,
}

/// [`CommitVerdictUseCase::attempt`] 1 回分の結末。
///
/// 楽観 version の競合だけを `Err` から切り出しているのは、**再試行の対象を名指しする**ため
/// である。競合したときにその試行が対象にしたステージを持ち帰らないと、2 回目が「そのときの
/// カーソル」へ再解決してしまい、競合相手が先に承認していた場合に報告されていない次ステージを
/// コミットしうる。
#[derive(Debug)]
enum AttemptOutcome {
    /// 決着した — コミットしたか、何もコミットしない成功だった。
    Settled(CommitOutcome),
    /// 楽観 version が競合した。
    Conflicted {
        /// この試行が対象にしたステージ（再試行はこれを名指しする）。
        target: StageSlug,
        /// ストアが返した競合そのもの（2 回目も競合したらこれを伝播する）。
        conflict: RepositoryError<IntentExecutionId>,
    },
}

impl<E: IntentExecutionRepository, I: IntentRepository, D: WorkflowDefinitionRepository>
    CommitVerdictUseCase<E, I, D>
{
    /// ポートの実装を 3 つ注入する。
    ///
    /// **ユースケースはリポジトリを保持し、`execute` の内部で使う** (改訂 10 のオーナー裁定)。
    /// 以前は Controller が `&Intent` を読んで渡していたが、あれは I8 — 読取専用ユースケース
    /// (`Next`) 専用のパターン — の誤適用だった (`coding-rules/use-case-rules.md` §4 の射程)。
    ///
    /// 定義ポート `D` は **Approve 段だけ**が使う（レビュー方針の解決 — b48 / 段 11）。
    /// 他の段は定義を読まないので、この注入が I/O を増やすのは承認経路だけである。
    #[must_use]
    pub const fn new(
        intent_execution_repository: E,
        intent_repository: I,
        workflow_definition_repository: D,
    ) -> CommitVerdictUseCase<E, I, D> {
        CommitVerdictUseCase {
            intent_execution_repository,
            intent_repository,
            workflow_definition_repository,
        }
    }

    /// 報告された結末を 1 つの遷移としてコミットする。
    ///
    /// `occurred_at` は呼出側が持つ時計の読みである — 集約は時計を持たない（NFR3.1）。
    ///
    /// # 引数は集約 ID と値オブジェクトだけである
    ///
    /// ユースケースの `execute` に**集約を渡さない** — 渡してよいのは集約 ID と値オブジェクト
    /// だけで、集約は保持するリポジトリから内部で取る
    /// （`coding-rules/use-case-rules.md` §2b、改訂 10 のオーナー裁定）。
    ///
    /// 内部フローは ① 実行を再構成 → ② その `intent_id` で計画を引く → ③ 集約のクエリに
    /// 判断を訊く → ④ 決まった段を打つ → ⑤ `store`。
    ///
    /// # 戻り値は「何をコミットしたか」の材料である
    ///
    /// b46 以前は CQS の Command の形（`Result<(), E>`）を採り、結果は投影後のリードモデルから
    /// 読み直す前提だった。13 段ガードの逐語（`Committed <subs> for "<slug>" (scope: <scope>)` /
    /// 拒否 12 形 / no-op 3 形）は**判断そのものが運ぶ材料**でしか組めないので、判断の答えを
    /// 呼出側へ返す。CQS の Command 規則を曲げているのではなく、この動詞の答えが
    /// 「状態の変化」ではなく「どの遷移を選んだか」だからである（集約コマンドが単一
    /// イベントを返すのと同じ位置づけ — `coding-rules/aggregate-commands.md`）。
    ///
    /// **何もコミットしない成功が 3 つある** — 既に開いているゲートへの再報告、カーソル
    /// 通過済みステージへの冪等な再報告（BR1.9）、完了済みワークフローへの再報告である。
    /// どれも `Ok(CommitOutcome::NoOp { .. })` であり、区別は変種が運ぶ。
    ///
    /// # `Conflict` は 1 回だけ再試行する
    ///
    /// 楽観 version の競合だけがこのユースケースの持つ唯一の再試行政策である
    /// （contract-design Q6 = A）。再試行は**再構成からやり直す** — 古い集約に `store` だけ
    /// 打ち直すのは、読んだ時点の版で書くという楽観ロックの意味そのものを壊す。2 回目も
    /// `Conflict` なら伝播する。
    ///
    /// **対象ステージは 1 回目が解決したものを名指しで引き継ぐ。** 名指し無しのまま再試行
    /// すると対象が「そのときのカーソル」に再解決され、競合相手が先に同じゲートを承認して
    /// いた場合に**報告されていない次のステージ**へ前進を打ってしまう。名指しすれば、その
    /// 状況は集約の forward 表が通過済み no-op に畳む。
    ///
    /// # Errors
    ///
    /// 実行の再構成・永続化の失敗（`Repository`）、計画の取得の失敗（`IntentRepository`）、
    /// 集約の判断による拒否（`Refused`）、判断が名指しした段の集約コマンドによる拒否
    /// （`Transition`）、この build に無い段（`UnwiredTransition`）を返す。集約とポートの
    /// 失敗は**そのまま伝播**する — 握り潰しも言い換えもしない。
    pub async fn execute(
        &mut self,
        execution_id: &IntentExecutionId,
        request: ReportRequest,
        occurred_at: DateTime<Utc>,
    ) -> Result<CommitOutcome, CommitError> {
        // 1 回目。競合しなければここで決着する。
        match self.attempt(execution_id, &request, occurred_at).await? {
            AttemptOutcome::Settled(outcome) => Ok(outcome),
            // 再試行は 1 回目が解決した対象を**名指しで**引き継ぐ（doc「対象ステージは…」）。
            AttemptOutcome::Conflicted { target, .. } => {
                let retried = ReportRequest::new(
                    request.verdict(),
                    Some(target),
                    request.user_input().map(str::to_string),
                    request.reason().map(str::to_string),
                    request.human_presence_guard(),
                );
                match self.attempt(execution_id, &retried, occurred_at).await? {
                    AttemptOutcome::Settled(outcome) => Ok(outcome),
                    AttemptOutcome::Conflicted { conflict, .. } => {
                        Err(CommitError::Repository(conflict))
                    }
                }
            }
        }
    }

    /// 再構成からコミットまでの 1 回分。競合したときはこれをもう 1 度だけ通す。
    ///
    /// # Errors
    ///
    /// 競合**以外**の失敗。楽観 version の競合だけは `Err` ではなく
    /// [`AttemptOutcome::Conflicted`] で返す — 呼出側が再試行の対象を名指しできるように、
    /// この試行が対象にしたステージを添えるためである。
    async fn attempt(
        &mut self,
        execution_id: &IntentExecutionId,
        request: &ReportRequest,
        occurred_at: DateTime<Utc>,
    ) -> Result<AttemptOutcome, CommitError> {
        // 再構成した集約は**ストアが刻んだ版を運んでいる**ので、書込へはそれをそのまま提示
        // する（ポート doc「楽観 version は集約が運ぶ」— 版は不透明なトークンであり
        // `aggregate.seq_nr()` から導いてはならない）。
        let mut aggregate = self
            .intent_execution_repository
            .find_by_id(execution_id)
            .await?;
        // 計画は**保持しているリポジトリから内部で取る**（改訂 10）。実行は intent を ID で
        // 参照するだけなので（`coding-rules/aggregate-references.md`）、その ID で引く。
        let intent = self
            .intent_repository
            .find_by_id(aggregate.intent_id())
            .await?;
        let scope = intent.scope().to_string();

        // 13 段ガードのうち状態で決まる段はすべてここで決着する（判断は集約に閉じる）。
        let (stage, steps) = match aggregate.report_dispatch(&intent, request)? {
            ReportDecision::NoOp(no_op) => {
                return Ok(AttemptOutcome::Settled(CommitOutcome::NoOp {
                    stage: Self::no_op_stage(&no_op).clone(),
                    scope,
                    no_op,
                }));
            }
            ReportDecision::Commit { stage, steps } => (stage, steps),
        };

        // 段 11 のレビュー方針は **Approve 段のときだけ**引く（他の段は定義を読まない）。
        let policy = if steps.contains(&TransitionStep::Approve) {
            self.review_policy(&intent, &stage).await?
        } else {
            None
        };
        let event = Self::commit_steps(
            &intent,
            &mut aggregate,
            &stage,
            &steps,
            policy.as_ref(),
            request,
            occurred_at,
        )?;
        match self
            .intent_execution_repository
            .store(&event, &aggregate)
            .await
        {
            Ok(()) => Ok(AttemptOutcome::Settled(CommitOutcome::Committed {
                stage,
                scope,
                steps,
            })),
            Err(conflict @ RepositoryError::Conflict { .. }) => Ok(AttemptOutcome::Conflicted {
                target: stage,
                conflict,
            }),
            Err(other) => Err(CommitError::Repository(other)),
        }
    }

    /// 判断が名指しした段を打つ（**イベントは 1 つ**）。
    ///
    /// 状態遷移を起こすのは列の**最後の段**である。先行する `GateStartRecovered` は監査の
    /// 見え方（`STAGE_AWAITING_APPROVAL` の `Recovered` 行）を決めるだけで、遷移そのものは
    /// 続く `Approve` の 1 イベント（BR1.3 — `[-]` からの承認）が担う。したがってここでも
    /// 「1 コマンド 1 イベント」は崩れない（`coding-rules/aggregate-commands.md`）。
    ///
    /// # Errors
    ///
    /// 集約がそのコマンドを拒否した（`Transition`）、この build に無い段だった
    /// （`UnwiredTransition`）。
    fn commit_steps(
        intent: &Intent,
        aggregate: &mut IntentExecution,
        stage: &StageSlug,
        steps: &[TransitionStep],
        policy: Option<&ReviewPolicy>,
        request: &ReportRequest,
        occurred_at: DateTime<Utc>,
    ) -> Result<IntentExecutionEvent, CommitError> {
        let (step, refused) = match steps {
            [TransitionStep::GateStart] => (
                TransitionStep::GateStart,
                aggregate.open_gate(intent, Vec::new(), occurred_at),
            ),
            [TransitionStep::Reject] => (
                TransitionStep::Reject,
                aggregate.reject_gate(intent, request.feedback().map(str::to_string), occurred_at),
            ),
            [TransitionStep::Revise] => (
                TransitionStep::Revise,
                aggregate.revise_stage(intent, occurred_at),
            ),
            [TransitionStep::Skip] => (
                TransitionStep::Skip,
                aggregate.skip_stage(
                    intent,
                    request.reason().unwrap_or_default().trim().to_string(),
                    occurred_at,
                ),
            ),
            [TransitionStep::Approve]
            | [TransitionStep::GateStartRecovered, TransitionStep::Approve] => (
                TransitionStep::Approve,
                // upstream の `approveArgs` は空文字列の `--user-input` を渡さない（JS の
                // truthy 判定）。空白だけの値は渡すので、`trim` ではなく空判定である。
                aggregate.approve_gate(
                    intent,
                    policy,
                    request
                        .user_input()
                        .filter(|text| !text.is_empty())
                        .map(str::to_string),
                    occurred_at,
                ),
            ),
            // `advance` / `complete-workflow` はこの build に対応する集約コマンドを持たない
            // （非ゲート完了のパイプラインは b42 で撤去 — #85 = A）。空列は判断が返さない
            // ので、名指しは先頭の段で足りる。
            other => {
                return Err(CommitError::UnwiredTransition {
                    step: other.first().copied().unwrap_or(TransitionStep::Advance),
                    stage: stage.clone(),
                });
            }
        };
        refused.map_err(|error| CommitError::Transition {
            step,
            stage: stage.clone(),
            error,
        })
    }

    /// 承認しようとしているステージのレビュー方針を定義から解決する（段 11 の材料）。
    ///
    /// `--review` override は intent が生値で運ぶ。閉集合で受けて鋳造しているので、ここで
    /// 復号に失敗するのは**壊れた歴史**である（`CorruptReviewOverride`）。
    ///
    /// # Errors
    ///
    /// 定義の再構成の失敗（`DefinitionRepository`）、定義がその slug を知らない
    /// （`UnknownDefinitionStage`）、壊れた `--review` 値（`CorruptReviewOverride`）。
    async fn review_policy(
        &self,
        intent: &Intent,
        stage: &StageSlug,
    ) -> Result<Option<ReviewPolicy>, CommitError> {
        let definition = self
            .workflow_definition_repository
            .find_by_id(intent.definition_id())
            .await
            .map_err(CommitError::DefinitionRepository)?;
        let review_override = intent
            .review()
            .map(|raw| {
                ReviewCapValue::parse(raw)
                    .map_err(|_| CommitError::CorruptReviewOverride(raw.to_string()))
            })
            .transpose()?;
        definition
            .review_policy(stage, intent.scope(), review_override)
            .map_err(|_| CommitError::UnknownDefinitionStage {
                stage: stage.clone(),
            })
    }

    /// no-op 3 形が名指しするステージ。
    const fn no_op_stage(no_op: &ReportNoOp) -> &StageSlug {
        match no_op {
            ReportNoOp::AlreadyAwaiting { stage }
            | ReportNoOp::AlreadyCompletedMovedOn { stage, .. }
            | ReportNoOp::WorkflowAlreadyCompleted { stage } => stage,
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

    /// 注入された定義ポートの実装（「定義を読むのは Approve 段だけ」の観測点 — b48）。
    #[cfg(test)]
    pub(crate) const fn workflow_definition_repository(&self) -> &D {
        &self.workflow_definition_repository
    }
}

#[cfg(test)]
mod tests {
    // panic! は「想定した変種でなければ即失敗」という検証用途で使っており、テスト失敗の
    // シグナルとして妥当なので許容する（集約のテストモジュールと同じ作法）。
    #![allow(clippy::panic)]

    use super::super::commit_error::CommitError;
    use super::super::commit_outcome::CommitOutcome;
    use super::super::commit_verdict_use_case::CommitVerdictUseCase;
    use super::super::port::RepositoryError;
    use super::super::test_support::{
        InMemoryIntentExecutionRepository, InMemoryIntentRepository,
        InMemoryWorkflowDefinitionRepository, absent_execution, at, definition,
        definition_with_reviewer, execution_id, genesis, genesis_with_review, slug,
        start_from_plan,
    };
    use chrono::{DateTime, Utc};
    use core_command_domain::orchestration::{
        CommandError, Intent, IntentExecution, IntentExecutionEvent, ReportNoOp, ReportRefusal,
        ReportRequest, TransitionStep, Verdict,
    };
    use core_command_domain::workflow_definition::{
        PhaseId, PlanAction, StageSlug, WorkflowDefinition,
    };

    /// カーソルが最初のゲート付きステージに立っている実行。
    ///
    /// 誕生 = 初期化完了済み（issue #76）以降、genesis がそのまま**この状態**である。
    fn at_the_first_gate(stage_count: usize) -> (Intent, IntentExecution) {
        let (intent, aggregate, _) = genesis(stage_count);
        (intent, aggregate)
    }

    /// テストの主体 — 3 本のポートを注入したユースケース。
    struct Subject {
        use_case: CommitVerdictUseCase<
            InMemoryIntentExecutionRepository,
            InMemoryIntentRepository,
            InMemoryWorkflowDefinitionRepository,
        >,
    }

    impl Subject {
        async fn execute(
            &mut self,
            request: ReportRequest,
            occurred_at: DateTime<Utc>,
        ) -> Result<CommitOutcome, CommitError> {
            self.use_case
                .execute(&execution_id(), request, occurred_at)
                .await
        }

        const fn intent_execution_repository(&self) -> &InMemoryIntentExecutionRepository {
            self.use_case.intent_execution_repository()
        }

        const fn intent_repository(&self) -> &InMemoryIntentRepository {
            self.use_case.intent_repository()
        }

        const fn workflow_definition_repository(&self) -> &InMemoryWorkflowDefinitionRepository {
            self.use_case.workflow_definition_repository()
        }
    }

    fn use_case(pair: (Intent, IntentExecution), version: usize) -> Subject {
        // 既定の定義はレビュアーを宣言しない — 段 11 の受領証は要らない (b48)。
        use_case_with(pair, version, definition)
    }

    /// 定義を差し替えられる形 (段 11 のレビュー受領証を見るテスト用)。
    fn use_case_with(
        pair: (Intent, IntentExecution),
        version: usize,
        definition_of: impl Fn(usize) -> WorkflowDefinition,
    ) -> Subject {
        let (intent, aggregate) = pair;
        let definition = definition_of(intent.stage_count());
        Subject {
            use_case: CommitVerdictUseCase::new(
                InMemoryIntentExecutionRepository::holding(aggregate, version),
                InMemoryIntentRepository::holding(intent),
                InMemoryWorkflowDefinitionRepository::holding(definition),
            ),
        }
    }

    /// 承認の報告（人間の選択つき — 段 13 を通す）。
    fn forward() -> ReportRequest {
        ReportRequest::new(
            Verdict::Forward,
            None,
            Some("Approve".to_string()),
            None,
            true,
        )
    }

    fn named(verdict: Verdict, stage: &StageSlug) -> ReportRequest {
        ReportRequest::new(
            verdict,
            Some(stage.clone()),
            Some("Approve".to_string()),
            None,
            true,
        )
    }

    /// **効果**の観測 — ストアが受理した唯一のイベントを取り出す。
    fn only_committed(
        intent_execution_repository: &InMemoryIntentExecutionRepository,
    ) -> &IntentExecutionEvent {
        let committed = intent_execution_repository.committed();
        assert_eq!(committed.len(), 1, "コミットは 1 件のはず");
        committed.first().expect("1 件ある")
    }

    /// 成功が名指しした段の綴り。
    fn steps_of(outcome: &CommitOutcome) -> Vec<&'static str> {
        match outcome {
            CommitOutcome::Committed { steps, .. } => {
                steps.iter().map(|step| step.subcommand()).collect()
            }
            CommitOutcome::NoOp { .. } => panic!("Committed を期待した: {outcome:?}"),
        }
    }

    // ---- 経路ごとの正常系（効果と戻り値の両方で観測する） ----

    #[tokio::test]
    async fn an_awaiting_approval_report_opens_the_gate() {
        let mut subject = use_case(at_the_first_gate(3), 1);
        let outcome = subject
            .execute(
                ReportRequest::new(Verdict::AwaitingApproval, None, None, None, true),
                at(),
            )
            .await
            .expect("in-progress のゲート付きステージは開ける");
        assert_eq!(steps_of(&outcome), ["gate-start"]);
        assert!(matches!(
            &outcome,
            CommitOutcome::Committed { stage, scope, .. } if *stage == slug(1) && scope == "classic"
        ));
        let IntentExecutionEvent::GateOpened(opened) =
            only_committed(subject.intent_execution_repository())
        else {
            panic!("GateOpened を期待した");
        };
        assert_eq!(opened.stage(), &slug(1));
    }

    #[tokio::test]
    async fn a_repeated_awaiting_approval_report_commits_nothing() {
        // upstream の `cli/report/awaiting-approval-repeat` は監査行も状態差分も空である。
        let (intent, mut aggregate) = at_the_first_gate(3);
        aggregate
            .open_gate(&intent, vec!["intent.md".to_string()], at())
            .expect("最初の開放は通る");
        let mut subject = use_case((intent, aggregate), 2);
        let outcome = subject
            .execute(
                ReportRequest::new(Verdict::AwaitingApproval, None, None, None, true),
                at(),
            )
            .await
            .expect("既に開いているゲートへの再報告は成功扱い");
        assert!(matches!(
            &outcome,
            CommitOutcome::NoOp {
                stage,
                no_op: ReportNoOp::AlreadyAwaiting { .. },
                ..
            } if *stage == slug(1)
        ));
        assert!(subject.intent_execution_repository().committed().is_empty());
        assert_eq!(
            subject.intent_execution_repository().store_attempts(),
            0,
            "書込を試みない"
        );
        assert_eq!(
            subject
                .intent_execution_repository()
                .version_of(&execution_id()),
            Some(2),
            "版も動かない"
        );
    }

    #[tokio::test]
    async fn a_forward_report_on_a_gated_stage_approves_the_gate() {
        let (intent, mut aggregate) = at_the_first_gate(3);
        aggregate
            .open_gate(&intent, Vec::new(), at())
            .expect("ゲートは開ける");
        let mut subject = use_case((intent, aggregate), 2);
        let outcome = subject
            .execute(forward(), at())
            .await
            .expect("ゲート付きステージは承認できる");
        assert_eq!(steps_of(&outcome), ["approve"]);
        let IntentExecutionEvent::GateApproved(approved) =
            only_committed(subject.intent_execution_repository())
        else {
            panic!("GateApproved を期待した");
        };
        assert_eq!(approved.stage(), &slug(1));
        assert_eq!(approved.user_input(), Some("Approve"));
    }

    #[tokio::test]
    async fn an_explicit_stage_recovers_the_gate_before_approving_in_one_event() {
        // `[-]` からの承認は 2 段を名乗るが、イベントは 1 本である（BR1.3）。
        let mut subject = use_case(at_the_first_gate(3), 1);
        let outcome = subject
            .execute(named(Verdict::Forward, &slug(1)), at())
            .await
            .expect("明示 --stage はゲートを開き直して承認する");
        assert_eq!(steps_of(&outcome), ["gate-start", "approve"]);
        assert!(matches!(
            only_committed(subject.intent_execution_repository()),
            IntentExecutionEvent::GateApproved(_)
        ));
    }

    #[tokio::test]
    async fn a_rejected_report_carries_the_feedback() {
        let mut subject = use_case(at_the_first_gate(3), 1);
        let outcome = subject
            .execute(
                ReportRequest::new(
                    Verdict::Rejected,
                    None,
                    None,
                    Some("Sharpen the testing posture.".to_string()),
                    true,
                ),
                at(),
            )
            .await
            .expect("ゲート付きステージは差し戻せる");
        assert_eq!(steps_of(&outcome), ["reject"]);
        let IntentExecutionEvent::GateRejected(rejected) =
            only_committed(subject.intent_execution_repository())
        else {
            panic!("GateRejected を期待した");
        };
        assert_eq!(rejected.feedback(), Some("Sharpen the testing posture."));
    }

    #[tokio::test]
    async fn a_revised_report_re_enters_the_gate() {
        let (intent, mut aggregate) = at_the_first_gate(3);
        aggregate
            .reject_gate(&intent, Some("直して".to_string()), at())
            .expect("差し戻しは通る");
        let mut subject = use_case((intent, aggregate), 2);
        let outcome = subject
            .execute(
                ReportRequest::new(Verdict::Revised, None, None, None, true),
                at(),
            )
            .await
            .expect("revising のステージはゲートへ再入できる");
        assert_eq!(steps_of(&outcome), ["revise"]);
        let IntentExecutionEvent::StageRevised(revised) =
            only_committed(subject.intent_execution_repository())
        else {
            panic!("StageRevised を期待した");
        };
        assert_eq!(revised.stage(), &slug(1));
    }

    #[tokio::test]
    async fn a_skipped_report_carries_the_trimmed_reason() {
        let (intent, aggregate, _) = start_from_plan(&[
            (PhaseId::Initialization, PlanAction::Execute, false),
            (PhaseId::Inception, PlanAction::Execute, true),
            (PhaseId::Inception, PlanAction::Execute, false),
        ]);
        let mut subject = use_case((intent, aggregate), 1);
        let outcome = subject
            .execute(
                ReportRequest::new(
                    Verdict::Skipped,
                    Some(slug(1)),
                    None,
                    Some("  Not applicable  ".to_string()),
                    true,
                ),
                at(),
            )
            .await
            .expect("CONDITIONAL なステージは読み飛ばせる");
        assert_eq!(steps_of(&outcome), ["skip"]);
        let IntentExecutionEvent::StageSkipped(skipped) =
            only_committed(subject.intent_execution_repository())
        else {
            panic!("StageSkipped を期待した");
        };
        // upstream も `flags.reason?.trim()` を渡す（ピン `:5620`）。
        assert_eq!(skipped.reason(), "Not applicable");
    }

    // ---- 冪等・no-op ----

    #[tokio::test]
    async fn a_re_report_of_a_stage_the_cursor_has_passed_commits_nothing() {
        // BR1.9 — カーソル通過済み completed への再報告は冪等。判断は集約の forward 表。
        let (intent, mut aggregate) = at_the_first_gate(3);
        aggregate
            .approve_gate(&intent, None, Some("Approve".to_string()), at())
            .expect("最初のゲートは承認できる");
        let mut subject = use_case((intent, aggregate), 2);
        let outcome = subject
            .execute(named(Verdict::Forward, &slug(1)), at())
            .await
            .expect("通過済み completed への再報告は冪等な成功");
        assert!(matches!(
            &outcome,
            CommitOutcome::NoOp {
                no_op: ReportNoOp::AlreadyCompletedMovedOn { stage, current },
                ..
            } if *stage == slug(1) && *current == slug(2)
        ));
        assert!(subject.intent_execution_repository().committed().is_empty());
        assert_eq!(subject.intent_execution_repository().store_attempts(), 0);
    }

    #[tokio::test]
    async fn a_re_report_of_a_completed_workflow_commits_nothing() {
        let (intent, mut aggregate) = at_the_first_gate(2);
        aggregate
            .approve_gate(&intent, None, Some("Approve".to_string()), at())
            .expect("最後のゲートは承認できる");
        let mut subject = use_case((intent, aggregate), 2);
        let outcome = subject
            .execute(forward(), at())
            .await
            .expect("完了済みへの再報告は冪等な成功");
        assert!(matches!(
            &outcome,
            CommitOutcome::NoOp {
                no_op: ReportNoOp::WorkflowAlreadyCompleted { stage },
                scope,
                ..
            } if *stage == slug(1) && scope == "classic"
        ));
    }

    #[tokio::test]
    async fn naming_the_cursor_explicitly_still_takes_the_normal_route() {
        let (intent, mut aggregate) = at_the_first_gate(3);
        aggregate
            .open_gate(&intent, Vec::new(), at())
            .expect("ゲートは開ける");
        let mut subject = use_case((intent, aggregate), 2);
        let outcome = subject
            .execute(named(Verdict::Forward, &slug(1)), at())
            .await
            .expect("カーソル自身を名指しした報告は通常経路");
        assert_eq!(steps_of(&outcome), ["approve"]);
    }

    // ---- 拒否の伝播 ----

    #[tokio::test]
    async fn a_report_that_names_a_stage_outside_the_plan_is_refused() {
        let mut subject = use_case(at_the_first_gate(3), 1);
        let unknown = StageSlug::parse("not-in-the-plan").expect("slug は文法内");
        let err = subject
            .execute(named(Verdict::Forward, &unknown), at())
            .await
            .expect_err("計画に無いステージは解決できない");
        assert!(matches!(
            err,
            CommitError::Refused(ReportRefusal::UnknownStage { named }) if named == "not-in-the-plan"
        ));
        assert!(subject.intent_execution_repository().committed().is_empty());
    }

    #[tokio::test]
    async fn a_report_that_names_a_stage_the_cursor_has_not_reached_is_refused() {
        let mut subject = use_case(at_the_first_gate(3), 1);
        let err = subject
            .execute(named(Verdict::Forward, &slug(2)), at())
            .await
            .expect_err("未着手のステージは前進の完了ではない");
        assert!(matches!(
            err,
            CommitError::Refused(ReportRefusal::StillPending { stage }) if stage == slug(2)
        ));
        assert!(subject.intent_execution_repository().committed().is_empty());
    }

    #[tokio::test]
    async fn the_human_presence_refusal_is_propagated_verbatim() {
        let mut subject = use_case(at_the_first_gate(3), 1);
        let err = subject
            .execute(
                ReportRequest::new(Verdict::Forward, Some(slug(1)), None, None, true),
                at(),
            )
            .await
            .expect_err("人間の選択が無ければ承認できない");
        assert!(matches!(
            err,
            CommitError::Refused(ReportRefusal::HumanPresence { stage, .. }) if stage == slug(1)
        ));
        assert!(subject.intent_execution_repository().committed().is_empty());
    }

    #[tokio::test]
    async fn a_step_this_build_cannot_commit_is_named_rather_than_re_read() {
        // 初期化ステージだけの縮退計画では、判断が `complete-workflow` を名指しする。
        // この build には対応する集約コマンドが無い（b42 — #85 = A）ので、読み替えずに断る。
        let (intent, aggregate, _) =
            start_from_plan(&[(PhaseId::Initialization, PlanAction::Execute, false)]);
        let mut subject = use_case((intent, aggregate), 1);
        let err = subject
            .execute(forward(), at())
            .await
            .expect_err("打てない段は成功にしない");
        assert!(matches!(
            err,
            CommitError::UnwiredTransition {
                step: TransitionStep::CompleteWorkflow,
                stage,
            } if stage == slug(0)
        ));
        assert!(subject.intent_execution_repository().committed().is_empty());
    }

    #[tokio::test]
    async fn a_command_the_aggregate_refuses_names_the_step_it_was_committing() {
        // 集約の判断を通っても、park 中なら集約コマンドは `NotRunning` で拒む。
        let (intent, mut aggregate) = at_the_first_gate(3);
        aggregate
            .open_gate(&intent, Vec::new(), at())
            .expect("ゲートは開ける");
        aggregate.park(&intent, at()).expect("park は通る");
        let mut subject = use_case((intent, aggregate), 3);
        let err = subject
            .execute(forward(), at())
            .await
            .expect_err("park 中の実行はコマンドを受理しない");
        assert!(matches!(
            err,
            CommitError::Transition {
                step: TransitionStep::Approve,
                ref stage,
                error: CommandError::NotRunning,
            } if *stage == slug(1)
        ));
        assert!(
            subject.intent_execution_repository().committed().is_empty(),
            "拒否されたコマンドは 1 バイトも書かない"
        );
    }

    // ---- 楽観 version の往復 ----

    #[tokio::test]
    async fn the_write_presents_the_version_the_rehydration_returned() {
        // `aggregate.seq_nr()` から導かない — 再構成が返した版そのものを渡す（ポート doc C3）。
        let pair = at_the_first_gate(3);
        assert_eq!(pair.1.seq_nr(), 1, "通番と版はたまたま一致させない");
        let mut subject = use_case(pair, 7);
        subject
            .execute(named(Verdict::Forward, &slug(1)), at())
            .await
            .expect("承認は通る");
        assert_eq!(
            subject
                .intent_execution_repository()
                .version_of(&execution_id()),
            Some(8),
            "版 7 を提示して書けたので、ストアは 8 を採番した"
        );
    }

    // ---- 異常系 ----

    #[tokio::test]
    async fn the_use_case_fetches_the_intent_itself_from_the_port() {
        let mut subject = use_case(at_the_first_gate(3), 7);
        assert_eq!(subject.intent_repository().lookups(), 0, "呼ぶ前は 0 回");
        subject
            .execute(named(Verdict::Forward, &slug(1)), at())
            .await
            .expect("ゲートは承認できる");
        assert_eq!(
            subject.intent_repository().lookups(),
            1,
            "1 試行につき 1 回引く"
        );
    }

    #[tokio::test]
    async fn a_missing_intent_is_propagated_from_its_own_port() {
        let (intent, aggregate) = at_the_first_gate(3);
        let mut use_case = CommitVerdictUseCase::new(
            InMemoryIntentExecutionRepository::holding(aggregate, 7),
            InMemoryIntentRepository::empty(),
            InMemoryWorkflowDefinitionRepository::holding(definition(3)),
        );
        let err = use_case
            .execute(&execution_id(), forward(), at())
            .await
            .expect_err("計画が無ければコミットできない");
        assert!(matches!(
            err,
            CommitError::IntentRepository(RepositoryError::NotFound { id }) if id == *intent.id()
        ));
    }

    #[tokio::test]
    async fn a_retry_fetches_the_intent_again() {
        let (intent, aggregate) = at_the_first_gate(3);
        let mut subject = Subject {
            use_case: CommitVerdictUseCase::new(
                InMemoryIntentExecutionRepository::holding_behind_concurrent_writes(
                    aggregate, 7, 1,
                ),
                InMemoryIntentRepository::holding(intent),
                InMemoryWorkflowDefinitionRepository::holding(definition(3)),
            ),
        };
        subject
            .execute(named(Verdict::Forward, &slug(1)), at())
            .await
            .expect("1 回だけ再試行すれば通る");
        assert_eq!(
            subject.intent_repository().lookups(),
            2,
            "2 試行なので 2 回引く"
        );
    }

    #[tokio::test]
    async fn a_missing_aggregate_is_reported_as_not_found() {
        let (intent, _, _) = genesis(3);
        let mut subject = CommitVerdictUseCase::new(
            InMemoryIntentExecutionRepository::empty(),
            InMemoryIntentRepository::holding(intent),
            InMemoryWorkflowDefinitionRepository::holding(definition(3)),
        );
        let err = subject
            .execute(&absent_execution(), forward(), at())
            .await
            .expect_err("ストアに無い集約は再構成できない");
        assert!(matches!(
            err,
            CommitError::Repository(RepositoryError::NotFound { id }) if id == absent_execution()
        ));
    }

    #[tokio::test]
    async fn a_first_conflict_is_retried_once_from_the_rehydration() {
        // 1 件の割り込み書込で 1 回目は競合し、2 回目で通る（contract-design Q6 = A）。
        let (intent, aggregate) = at_the_first_gate(3);
        let mut subject = Subject {
            use_case: CommitVerdictUseCase::new(
                InMemoryIntentExecutionRepository::holding_behind_concurrent_writes(
                    aggregate, 7, 1,
                ),
                InMemoryIntentRepository::holding(intent),
                InMemoryWorkflowDefinitionRepository::holding(definition(3)),
            ),
        };
        let outcome = subject
            .execute(named(Verdict::Forward, &slug(1)), at())
            .await
            .expect("1 回だけ再試行すれば通る");
        assert_eq!(steps_of(&outcome), ["gate-start", "approve"]);
        assert!(matches!(
            only_committed(subject.intent_execution_repository()),
            IntentExecutionEvent::GateApproved(_)
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
            Some(9)
        );
    }

    #[tokio::test]
    async fn a_retry_after_a_competitor_committed_the_same_gate_commits_nothing() {
        // 競合相手が先に同じゲートを承認してカーソルが動いたケース。再試行が対象を名指しし
        // 直さないと、報告されていない次ステージへ前進を打ってしまう。
        let (intent, held) = at_the_first_gate(3);
        let mut advanced = held.clone();
        advanced
            .approve_gate(&intent, None, Some("Approve".to_string()), at())
            .expect("競合相手の承認は通る");
        assert_ne!(
            advanced.cursor(),
            held.cursor(),
            "相手の承認でカーソルが動いている前提のテストである"
        );

        let mut subject = Subject {
            use_case: CommitVerdictUseCase::new(
                InMemoryIntentExecutionRepository::holding_behind_a_competing_commit(
                    held, advanced, 7,
                ),
                InMemoryIntentRepository::holding(intent),
                InMemoryWorkflowDefinitionRepository::holding(definition(3)),
            ),
        };
        let outcome = subject
            .execute(named(Verdict::Forward, &slug(1)), at())
            .await
            .expect("通過済みになった報告は冪等な成功");
        assert!(matches!(
            &outcome,
            CommitOutcome::NoOp {
                no_op: ReportNoOp::AlreadyCompletedMovedOn { .. },
                ..
            }
        ));
        assert!(
            subject.intent_execution_repository().committed().is_empty(),
            "次ステージを勝手にコミットしない"
        );
        assert_eq!(
            subject.intent_execution_repository().store_attempts(),
            1,
            "書込は 1 回目の失敗だけ — 再試行は forward 表の no-op に畳まれる"
        );
    }

    #[tokio::test]
    async fn a_second_conflict_is_propagated_without_a_further_retry() {
        // 2 件の割り込み書込。2 回目も競合したら伝播する — 3 回目は無い。
        let (intent, aggregate) = at_the_first_gate(3);
        let mut subject = Subject {
            use_case: CommitVerdictUseCase::new(
                InMemoryIntentExecutionRepository::holding_behind_concurrent_writes(
                    aggregate, 7, 2,
                ),
                InMemoryIntentRepository::holding(intent),
                InMemoryWorkflowDefinitionRepository::holding(definition(3)),
            ),
        };
        let err = subject
            .execute(named(Verdict::Forward, &slug(1)), at())
            .await
            .expect_err("2 回目も競合したら伝播する");
        assert!(matches!(
            err,
            CommitError::Repository(RepositoryError::Conflict {
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
    // ---- b48: 段 11 のレビュー受領証 (#51 / B10) ----

    /// 索引 1 にレビュアーを宣言した定義 (クラス宣言なし = adversarial 扱い)。
    fn reviewed_definition(stage_count: usize) -> WorkflowDefinition {
        definition_with_reviewer(stage_count, 1, "aidlc-quality-agent", None, None, None)
    }

    /// 定義を読むのは **Approve 段だけ**である — 他の段は I/O を増やさない。
    #[tokio::test]
    async fn only_the_approve_step_reads_the_definition() {
        // 差し戻し（Reject 段）は定義を読まない。
        let mut subject = use_case(at_the_first_gate(3), 7);
        subject
            .execute(
                ReportRequest::new(
                    Verdict::Rejected,
                    Some(slug(1)),
                    None,
                    Some("直せ".to_string()),
                    true,
                ),
                at(),
            )
            .await
            .expect("差し戻しは通る");
        assert_eq!(
            subject.workflow_definition_repository().lookups(),
            0,
            "Reject 段は定義を読まない"
        );

        // 承認（Approve 段）は 1 試行につき 1 回だけ読む。
        let mut subject = use_case(at_the_first_gate(3), 7);
        subject
            .execute(named(Verdict::Forward, &slug(1)), at())
            .await
            .expect("承認は通る");
        assert_eq!(
            subject.workflow_definition_repository().lookups(),
            1,
            "Approve 段は 1 試行につき 1 回だけ読む"
        );
    }

    /// レビュアーを宣言したステージは、受領証が無ければ段 11 で拒まれる。
    #[tokio::test]
    async fn a_reviewer_bearing_stage_without_a_receipt_is_refused_at_the_approval_gate() {
        let mut subject = use_case_with(at_the_first_gate(3), 7, reviewed_definition);

        let err = subject
            .execute(named(Verdict::Forward, &slug(1)), at())
            .await
            .expect_err("受領証が無ければ承認できない");

        assert!(
            matches!(
                &err,
                CommitError::Transition {
                    step: TransitionStep::Approve,
                    stage,
                    error: CommandError::ReviewReceiptMissing { reviewer, .. },
                } if stage == &slug(1) && reviewer == "aidlc-quality-agent"
            ),
            "段 11 の材料をそのまま伝播する: {err:?}"
        );
        assert!(
            subject.intent_execution_repository().committed().is_empty(),
            "拒まれた承認は何もコミットしない"
        );
    }

    /// 実効クラスが `none` に落ちた実行は受領証を要求しない
    /// （`--review none` の override — upstream `verifyReviewerPrecondition` `:1810-1812`）。
    #[tokio::test]
    async fn an_effective_none_override_waives_the_receipt() {
        let (intent, aggregate, _) = genesis_with_review(3, "none");
        let mut subject = use_case_with((intent, aggregate), 7, reviewed_definition);

        subject
            .execute(named(Verdict::Forward, &slug(1)), at())
            .await
            .expect("実効 none は受領証を要らない");
    }

    /// 壊れた `--review` の値は「壊れた歴史」として surface する。
    #[tokio::test]
    async fn a_corrupt_review_override_is_surfaced_rather_than_defaulted() {
        let (intent, aggregate, _) = genesis_with_review(3, "Adversarial");
        let mut subject = use_case_with((intent, aggregate), 7, reviewed_definition);

        let err = subject
            .execute(named(Verdict::Forward, &slug(1)), at())
            .await
            .expect_err("閉集合の外は既定へ落とさない");
        assert!(
            matches!(&err, CommitError::CorruptReviewOverride(raw) if raw == "Adversarial"),
            "{err:?}"
        );
    }

    /// 定義がそのステージを知らなければ、承認は既定へ落とさず断る。
    #[tokio::test]
    async fn a_stage_the_definition_does_not_know_refuses_the_approval() {
        // 定義は 1 段だけ (`stage-0`) — 実行の計画 (3 段) のカーソル `stage-1` を知らない。
        // 判断 (`report_dispatch`) は計画で解決するので通り、段 11 の定義照会だけが落ちる。
        let mut subject = use_case_with(at_the_first_gate(3), 7, |_| definition(1));

        let err = subject
            .execute(named(Verdict::Forward, &slug(1)), at())
            .await
            .expect_err("定義に無いステージは承認できない");

        assert!(
            matches!(&err, CommitError::UnknownDefinitionStage { stage } if stage == &slug(1)),
            "{err:?}"
        );
    }

    /// 定義ポートの失敗はそのまま伝播する（握り潰さない）。
    #[tokio::test]
    async fn a_definition_port_failure_is_propagated_verbatim() {
        let (intent, aggregate) = at_the_first_gate(3);
        let mut subject = Subject {
            use_case: CommitVerdictUseCase::new(
                InMemoryIntentExecutionRepository::holding(aggregate, 7),
                InMemoryIntentRepository::holding(intent),
                InMemoryWorkflowDefinitionRepository::empty(),
            ),
        };
        let err = subject
            .execute(named(Verdict::Forward, &slug(1)), at())
            .await
            .expect_err("定義が無ければ承認できない");
        assert!(
            matches!(
                &err,
                CommitError::DefinitionRepository(RepositoryError::NotFound { .. })
            ),
            "{err:?}"
        );
    }
}
