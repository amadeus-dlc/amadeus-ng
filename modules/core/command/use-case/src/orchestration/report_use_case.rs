//! `ReportUseCase` — `report` 動詞のユースケース (FR2.1)。

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    IntentId, StageIndex, WorkflowExecution, WorkflowExecutionEvent,
};
use core_command_domain::workflow_definition::StageSlug;
use core_command_domain::workspace::CheckboxState;

use super::report_error::ReportError;
use super::report_outcome::ReportOutcome;
use super::reported_verdict::{ReportedTransition, ReportedVerdict};
use super::workflow_execution_repository::WorkflowExecutionRepository;

/// `report` — コンダクタが報告した結末を 1 つの遷移としてコミットする。
///
/// 定型は 4 手である: **`find_by_id` で集約を再構成 → 集約コマンドで判断 → `store` で保存 →
/// 型付きの結果を返す**。
///
/// # ここに無いもの
///
/// - **業務判断**。前提の検査 (受理状態・ゲートの有無・checkbox の前提集合・読み飛ばし可否・
///   通過済み判定) はすべて集約が持つ。ここにあるのは「どの集約コマンドを打つか」を集約の
///   クエリに訊いて決めるフロー制御だけである (`coding-rules/tell-dont-ask.md` — 判断は
///   状態の所有者へ)。
/// - **文言**。「Committed approve for "..."」のような逐語は出す側 (合成ルート U7 の
///   Presenter) が [`ReportOutcome`] の材料から組む。
/// - **リードモデルの更新**。`aidlc-state.md` と監査シャードを最新化する `ReadModelUpdater` を
///   起動するのは合成ルート (U7) である。コマンド側のユースケースはクエリ側を知らない
///   (`coding-rules/cqrs-boundaries.md` — 境界はクレート分離で物理強制されている)。
/// - **再試行の政策**。`Conflict` も含めて再試行しない (ポート doc の C3 ③)。
/// - **CLI のフラグ解析**。綴りの揺れ (`approved` / `completed` / `complete` / `done`) の受理は
///   U7 が [`ReportedVerdict`] を組む時点で畳む。
///
/// # 束縛はスタティック
///
/// `dyn` は使わない (`coding-rules/use-case-rules.md` §2)。結線 (実物 / インメモリの選択) は
/// 合成ルートだけが行い、ユースケースはポートの trait しか知らない。
#[derive(Debug)]
pub struct ReportUseCase<R: WorkflowExecutionRepository> {
    repository: R,
}

impl<R: WorkflowExecutionRepository> ReportUseCase<R> {
    /// ポートの実装を注入する。
    #[must_use]
    pub const fn new(repository: R) -> ReportUseCase<R> {
        ReportUseCase { repository }
    }

    /// 報告された結末を 1 つの遷移としてコミットする。
    ///
    /// `stage` は報告が名指ししたステージ (`None` はカーソル)。カーソル以外を名指しした報告は
    /// **通過済み completed への再報告**としてのみ受理し、集約の `stale_report` に判断を委ねる
    /// (BR1.9)。
    ///
    /// `occurred_at` は呼出側が持つ時計の読みである — 集約は時計を持たない (NFR3.1)。
    ///
    /// # 戻り値がある `&mut self` について
    ///
    /// CQS の既定 (Command は戻り値なし) から外れるが、これは集約コマンドが
    /// `&mut self -> Result<WorkflowExecutionEvent, _>` である理由と同じである — イベント
    /// ソーシングでは 1 コマンドが 1 イベントを**返す**ことが契約であり、コミットした事実と
    /// その材料を呼出側へ渡さないと Presenter が何も描けない。分離すると 2 つ目の呼出が
    /// 別トランザクションになり、コミットの有無と結果が食い違いうる
    /// (`coding-rules/command-query-separation.md` の判定フロー 3 = No)。
    ///
    /// # Errors
    ///
    /// 再構成・永続化の失敗 (`Repository`)、集約による拒否 (`Command`)、計画に無いステージの
    /// 名指し (`UnknownStage`) を返す。集約とポートの失敗は**そのまま伝播**する — 握り潰しも
    /// 言い換えも再試行もしない。
    pub async fn execute(
        &mut self,
        intent_id: &IntentId,
        stage: Option<&StageSlug>,
        reported: ReportedVerdict,
        occurred_at: DateTime<Utc>,
    ) -> Result<ReportOutcome, ReportError> {
        // 再開は集約に届かない (型がそう言っている — `ReportedVerdict` の doc)。再構成もしない。
        let ReportedVerdict::Transition(transition) = reported else {
            return Ok(ReportOutcome::ResumeRouting);
        };

        let rehydrated = self.repository.find_by_id(intent_id).await?;
        // 版は再構成が返した値**そのもの**を握る。`aggregate.seq_nr()` から導いてはならない
        // (ポート doc の 3 か条 — 版は不透明なトークンである)。
        let expected_version = rehydrated.version();
        let mut aggregate = rehydrated.into_aggregate();

        if let Some(named) = stage
            && let Some(outcome) = Self::stale_re_report(&aggregate, named)?
        {
            return Ok(outcome);
        }

        let cursor = aggregate.cursor();
        if let Some(outcome) = Self::gate_already_open(&aggregate, cursor, &transition) {
            return Ok(outcome);
        }

        let event = Self::command(&mut aggregate, cursor, transition, occurred_at)?;
        self.repository
            .store(&event, &aggregate, expected_version)
            .await?;
        Ok(ReportOutcome::Committed { event })
    }

    /// 名指しされたステージがカーソルの手前なら、集約に通過済み判定を委ねる (BR1.9)。
    ///
    /// カーソル自身を名指しした報告は通常経路なので `None` を返す。判断は集約の
    /// `stale_report` が持ち、ここがしているのは slug から位置への解決だけである。
    fn stale_re_report(
        aggregate: &WorkflowExecution,
        named: &StageSlug,
    ) -> Result<Option<ReportOutcome>, ReportError> {
        let target = Self::locate(aggregate, named).ok_or_else(|| ReportError::UnknownStage {
            stage: named.clone(),
        })?;
        if target == aggregate.cursor() {
            return Ok(None);
        }
        let decision = aggregate.stale_report(target)?;
        Ok(Some(ReportOutcome::AlreadyDone {
            stage: named.clone(),
            decision,
        }))
    }

    /// 解決済み計画の中での位置 (計画に無ければ `None`)。
    ///
    /// 集約の読取モデル (`stages` / `stage_index`) だけで完結する**参照**であって判断ではない
    /// — 前提の判定 (通過済み completed か) は集約の `stale_report` が持つ。
    fn locate(aggregate: &WorkflowExecution, named: &StageSlug) -> Option<StageIndex> {
        let position = aggregate
            .stages()
            .iter()
            .position(|entry| entry.slug() == named)?;
        aggregate.stage_index(position)
    }

    /// 既に開いているゲートへの `awaiting-approval` 再報告は、何もコミットせず成功扱いにする。
    ///
    /// upstream の `cli/report/awaiting-approval-repeat` は監査行も状態差分も空である。集約に
    /// 打てば `CheckboxPrecondition` で拒否されるが、それは**失敗ではない**ので、報告された語と
    /// 現在の印を突き合わせるフロー制御でここを分ける。
    fn gate_already_open(
        aggregate: &WorkflowExecution,
        cursor: StageIndex,
        transition: &ReportedTransition,
    ) -> Option<ReportOutcome> {
        if !matches!(transition, ReportedTransition::AwaitingApproval { .. })
            || aggregate.checkbox(cursor) != Some(CheckboxState::AwaitingApproval)
        {
            return None;
        }
        aggregate
            .stages()
            .get(cursor.to_usize())
            .map(|entry| ReportOutcome::GateAlreadyOpen {
                stage: entry.slug().clone(),
            })
    }

    /// 報告された結末に対応する集約コマンドを 1 つ打つ。
    ///
    /// `Forward` がどちらのコマンドになるかは、報告された語ではなく**ステージの性質**で決まる
    /// — ゲート付きなら承認、非ゲート (initialization) なら完了である (BR1.3)。どちらを打つかを
    /// 集約の `gated` クエリに訊いて決めるのはフロー制御であって、業務判断の複製ではない。
    fn command(
        aggregate: &mut WorkflowExecution,
        cursor: StageIndex,
        transition: ReportedTransition,
        occurred_at: DateTime<Utc>,
    ) -> Result<WorkflowExecutionEvent, ReportError> {
        let event = match transition {
            ReportedTransition::AwaitingApproval { artifacts } => {
                aggregate.open_gate(artifacts, occurred_at)
            }
            ReportedTransition::Forward { user_input } => {
                // カーソルは不変条件により常に範囲内なので `None` は起きない。起きたとしても
                // 非ゲート扱いに畳めば `complete_stage` が `InvalidTarget` で拒否するので、
                // ここで panic する理由はない (NFR4.3 — 集約の `commit` と同じ作法)。
                if aggregate.gated(cursor).unwrap_or(false) {
                    aggregate.approve_gate(user_input, occurred_at)
                } else {
                    aggregate.complete_stage(occurred_at)
                }
            }
            ReportedTransition::Rejected { feedback } => {
                aggregate.reject_gate(feedback, occurred_at)
            }
            ReportedTransition::Revised => aggregate.revise_stage(occurred_at),
            ReportedTransition::Skipped { reason } => aggregate.skip_stage(reason, occurred_at),
        };
        Ok(event?)
    }

    /// 注入されたポート実装 (テストがコミットの有無を観測するための継ぎ目)。
    #[cfg(test)]
    pub(crate) const fn repository(&self) -> &R {
        &self.repository
    }
}

#[cfg(test)]
mod tests {
    // panic! は「想定した変種でなければ即失敗」という検証用途で使っており、テスト失敗の
    // シグナルとして妥当なので許容する (集約のテストモジュールと同じ作法)。
    #![allow(clippy::panic)]

    use super::super::report_error::ReportError;
    use super::super::report_outcome::ReportOutcome;
    use super::super::report_use_case::ReportUseCase;
    use super::super::reported_verdict::{ReportedTransition, ReportedVerdict};
    use super::super::repository_error::RepositoryError;
    use super::super::test_support::{
        InMemoryWorkflowExecutionRepository, absent_intent, at, genesis, intent, slug,
        start_from_plan,
    };
    use core_command_domain::orchestration::{
        CommandError, NextDecision, PhaseBoundary, Verdict, WorkflowExecution,
        WorkflowExecutionEvent,
    };
    use core_command_domain::workflow_definition::{PhaseId, PlanAction, StageSlug};
    use core_command_domain::workspace::CheckboxState;

    /// 索引 0 (initialization) を完了させ、カーソルを最初のゲート付きステージへ進めた集約。
    fn at_the_first_gate(stage_count: usize) -> WorkflowExecution {
        let (mut aggregate, _) = genesis(stage_count);
        aggregate
            .complete_stage(at())
            .expect("初期化ステージは非ゲートなので完了できる");
        aggregate
    }

    fn use_case(
        aggregate: WorkflowExecution,
        version: usize,
    ) -> ReportUseCase<InMemoryWorkflowExecutionRepository> {
        ReportUseCase::new(InMemoryWorkflowExecutionRepository::holding(
            aggregate, version,
        ))
    }

    fn forward() -> ReportedVerdict {
        ReportedVerdict::Transition(ReportedTransition::Forward {
            user_input: Some("Approve".to_string()),
        })
    }

    fn committed_event(outcome: &ReportOutcome) -> &WorkflowExecutionEvent {
        match outcome {
            ReportOutcome::Committed { event } => event,
            other => panic!("コミットされていない: {other:?}"),
        }
    }

    // ---- 経路ごとの正常系 ----

    #[tokio::test]
    async fn an_awaiting_approval_report_opens_the_gate() {
        let mut subject = use_case(at_the_first_gate(3), 1);
        let outcome = subject
            .execute(
                &intent(),
                None,
                ReportedVerdict::Transition(ReportedTransition::AwaitingApproval {
                    artifacts: vec!["intent.md".to_string()],
                }),
                at(),
            )
            .await
            .expect("in-progress のゲート付きステージは開ける");
        let WorkflowExecutionEvent::GateOpened(opened) = committed_event(&outcome) else {
            panic!("GateOpened を期待した");
        };
        assert_eq!(opened.stage(), &slug(1));
        assert_eq!(opened.artifacts(), ["intent.md".to_string()]);
        assert_eq!(subject.repository().committed().len(), 1);
    }

    #[tokio::test]
    async fn a_repeated_awaiting_approval_report_commits_nothing() {
        // upstream の `cli/report/awaiting-approval-repeat` は監査行も状態差分も空である。
        let mut aggregate = at_the_first_gate(3);
        aggregate
            .open_gate(vec!["intent.md".to_string()], at())
            .expect("最初の開放は通る");
        let mut subject = use_case(aggregate, 2);
        let outcome = subject
            .execute(
                &intent(),
                None,
                ReportedVerdict::Transition(ReportedTransition::AwaitingApproval {
                    artifacts: vec!["intent.md".to_string()],
                }),
                at(),
            )
            .await
            .expect("既に開いているゲートへの再報告は成功扱い");
        assert_eq!(outcome, ReportOutcome::GateAlreadyOpen { stage: slug(1) });
        assert!(subject.repository().committed().is_empty());
        assert_eq!(subject.repository().version(), 2, "版も動かない");
    }

    #[tokio::test]
    async fn a_forward_report_on_a_gated_stage_approves_the_gate() {
        let mut subject = use_case(at_the_first_gate(3), 1);
        let outcome = subject
            .execute(&intent(), None, forward(), at())
            .await
            .expect("ゲート付きステージは承認できる");
        let WorkflowExecutionEvent::GateApproved(approved) = committed_event(&outcome) else {
            panic!("GateApproved を期待した");
        };
        assert_eq!(approved.stage(), &slug(1));
        assert_eq!(approved.user_input(), Some("Approve"));
        assert_eq!(approved.next_stage(), Some(&slug(2)));
    }

    #[tokio::test]
    async fn a_forward_report_on_an_ungated_stage_completes_the_stage() {
        // カーソルは索引 0 (initialization = 非ゲート)。どちらのコマンドを打つかは集約の
        // `gated` クエリで決まる。
        let (aggregate, _) = genesis(3);
        let mut subject = use_case(aggregate, 1);
        let outcome = subject
            .execute(&intent(), None, forward(), at())
            .await
            .expect("非ゲートステージは完了できる");
        let WorkflowExecutionEvent::StageCompleted(completed) = committed_event(&outcome) else {
            panic!("StageCompleted を期待した");
        };
        assert_eq!(completed.stage(), &slug(0));
        assert_eq!(completed.next_stage(), Some(&slug(1)));
    }

    #[tokio::test]
    async fn a_rejected_report_carries_the_feedback() {
        let mut subject = use_case(at_the_first_gate(3), 1);
        let outcome = subject
            .execute(
                &intent(),
                None,
                ReportedVerdict::Transition(ReportedTransition::Rejected {
                    feedback: Some("Sharpen the testing posture.".to_string()),
                }),
                at(),
            )
            .await
            .expect("ゲート付きステージは差し戻せる");
        let WorkflowExecutionEvent::GateRejected(rejected) = committed_event(&outcome) else {
            panic!("GateRejected を期待した");
        };
        assert_eq!(rejected.feedback(), Some("Sharpen the testing posture."));
        assert_eq!(rejected.revision_count(), 1);
    }

    #[tokio::test]
    async fn a_revised_report_re_enters_the_gate() {
        let mut aggregate = at_the_first_gate(3);
        aggregate
            .reject_gate(Some("直して".to_string()), at())
            .expect("差し戻しは通る");
        let mut subject = use_case(aggregate, 2);
        let outcome = subject
            .execute(
                &intent(),
                None,
                ReportedVerdict::Transition(ReportedTransition::Revised),
                at(),
            )
            .await
            .expect("revising のステージはゲートへ再入できる");
        let WorkflowExecutionEvent::StageRevised(revised) = committed_event(&outcome) else {
            panic!("StageRevised を期待した");
        };
        assert_eq!(revised.stage(), &slug(1));
    }

    #[tokio::test]
    async fn a_skipped_report_carries_the_reason() {
        let (mut aggregate, _) = start_from_plan(&[
            (PhaseId::Initialization, PlanAction::Execute, false),
            (PhaseId::Inception, PlanAction::Execute, true),
            (PhaseId::Inception, PlanAction::Execute, false),
        ]);
        aggregate.complete_stage(at()).expect("初期化は完了できる");
        let mut subject = use_case(aggregate, 1);
        let outcome = subject
            .execute(
                &intent(),
                None,
                ReportedVerdict::Transition(ReportedTransition::Skipped {
                    reason: "Not applicable".to_string(),
                }),
                at(),
            )
            .await
            .expect("CONDITIONAL なステージは読み飛ばせる");
        let WorkflowExecutionEvent::StageSkipped(skipped) = committed_event(&outcome) else {
            panic!("StageSkipped を期待した");
        };
        assert_eq!(skipped.reason(), "Not applicable");
        assert_eq!(skipped.next_stage(), Some(&slug(2)));
    }

    #[tokio::test]
    async fn a_resume_report_routes_without_touching_the_aggregate() {
        // 集約を持たないストアでも成功する = 再構成すらしていないことの証拠である。
        let mut subject = ReportUseCase::new(InMemoryWorkflowExecutionRepository::empty());
        let outcome = subject
            .execute(&intent(), None, ReportedVerdict::Resumed, at())
            .await
            .expect("resume はルーティングのみ");
        assert_eq!(outcome, ReportOutcome::ResumeRouting);
        assert!(subject.repository().committed().is_empty());
    }

    // ---- 冪等・no-op ----

    #[tokio::test]
    async fn a_re_report_of_a_stage_the_cursor_has_passed_commits_nothing() {
        // BR1.9 — カーソル通過済み completed への再報告は冪等 done。
        let mut subject = use_case(at_the_first_gate(3), 2);
        let outcome = subject
            .execute(&intent(), Some(&slug(0)), forward(), at())
            .await
            .expect("通過済み completed への再報告は冪等");
        assert_eq!(
            outcome,
            ReportOutcome::AlreadyDone {
                stage: slug(0),
                decision: NextDecision::Done,
            }
        );
        assert!(subject.repository().committed().is_empty());
        assert_eq!(subject.repository().version(), 2);
    }

    #[tokio::test]
    async fn naming_the_cursor_explicitly_still_takes_the_normal_route() {
        let mut subject = use_case(at_the_first_gate(3), 1);
        let outcome = subject
            .execute(&intent(), Some(&slug(1)), forward(), at())
            .await
            .expect("カーソル自身を名指しした報告は通常経路");
        assert!(matches!(
            committed_event(&outcome),
            WorkflowExecutionEvent::GateApproved(_)
        ));
    }

    #[tokio::test]
    async fn a_report_that_names_a_stage_outside_the_plan_is_refused() {
        let mut subject = use_case(at_the_first_gate(3), 1);
        let unknown = StageSlug::parse("not-in-the-plan").expect("slug は文法内");
        let err = subject
            .execute(&intent(), Some(&unknown), forward(), at())
            .await
            .expect_err("計画に無いステージは解決できない");
        assert_eq!(err, ReportError::UnknownStage { stage: unknown });
        assert!(subject.repository().committed().is_empty());
    }

    #[tokio::test]
    async fn a_report_that_names_a_stage_the_cursor_has_not_reached_is_refused_by_the_aggregate() {
        let mut subject = use_case(at_the_first_gate(3), 1);
        let err = subject
            .execute(&intent(), Some(&slug(2)), forward(), at())
            .await
            .expect_err("未着手のステージは通過済み completed ではない");
        let stage = at_the_first_gate(3)
            .stage_index(2)
            .expect("索引 2 は範囲内");
        assert_eq!(err, ReportError::Command(CommandError::NotStale(stage)));
    }

    // ---- 楽観 version の往復 ----

    #[tokio::test]
    async fn the_write_presents_the_version_the_rehydration_returned() {
        // `aggregate.seq_nr()` から導かない — 再構成が返した版そのものを渡す (ポート doc C3)。
        let aggregate = at_the_first_gate(3);
        assert_eq!(aggregate.seq_nr(), 2, "通番と版はたまたま一致させない");
        let mut subject = use_case(aggregate, 7);
        subject
            .execute(&intent(), None, forward(), at())
            .await
            .expect("承認は通る");
        assert_eq!(
            subject.repository().version(),
            8,
            "版 7 を提示して書けたので、ストアは 8 を採番した"
        );
    }

    // ---- 異常系 ----

    #[tokio::test]
    async fn a_missing_aggregate_is_reported_as_not_found() {
        let mut subject = ReportUseCase::new(InMemoryWorkflowExecutionRepository::empty());
        let err = subject
            .execute(&absent_intent(), None, forward(), at())
            .await
            .expect_err("ストアに無い集約は再構成できない");
        assert_eq!(
            err,
            ReportError::Repository(RepositoryError::NotFound {
                intent_id: absent_intent(),
            })
        );
    }

    #[tokio::test]
    async fn a_write_that_lost_the_race_is_reported_as_a_conflict() {
        // 再構成が返した版が古い = 読んだ後に別の書き手が書いた。再試行はしない (C3 ③)。
        let mut subject = ReportUseCase::new(
            InMemoryWorkflowExecutionRepository::holding_after_a_concurrent_write(
                at_the_first_gate(3),
                7,
            ),
        );
        let err = subject
            .execute(&intent(), None, forward(), at())
            .await
            .expect_err("古い版の提示は競合する");
        assert_eq!(
            err,
            ReportError::Repository(RepositoryError::Conflict {
                expected: 6,
                actual: 7,
            })
        );
        assert!(subject.repository().committed().is_empty());
    }

    #[tokio::test]
    async fn a_command_the_aggregate_refuses_is_propagated_verbatim() {
        // in-progress のステージは revise できない。ユースケースは言い換えも握り潰しもしない。
        let aggregate = at_the_first_gate(3);
        let stage = aggregate.cursor();
        let mut subject = use_case(aggregate, 1);
        let err = subject
            .execute(
                &intent(),
                None,
                ReportedVerdict::Transition(ReportedTransition::Revised),
                at(),
            )
            .await
            .expect_err("revising 以外はゲートへ再入できない");
        assert_eq!(
            err,
            ReportError::Command(CommandError::CheckboxPrecondition {
                stage,
                actual: CheckboxState::InProgress,
            })
        );
        assert!(
            subject.repository().committed().is_empty(),
            "拒否されたコマンドは 1 バイトも書かない"
        );
    }

    // ---- 層の境界 ----

    #[tokio::test]
    async fn the_phase_boundary_comes_from_the_aggregate_not_from_the_use_case() {
        // 裁定 2 — ユースケースは境界を導出しないし、渡しもしない。
        let (mut aggregate, _) = start_from_plan(&[
            (PhaseId::Initialization, PlanAction::Execute, false),
            (PhaseId::Ideation, PlanAction::Execute, false),
            (PhaseId::Inception, PlanAction::Execute, false),
        ]);
        aggregate.complete_stage(at()).expect("初期化は完了できる");
        let mut subject = use_case(aggregate, 1);
        let outcome = subject
            .execute(&intent(), None, forward(), at())
            .await
            .expect("承認は通る");
        let WorkflowExecutionEvent::GateApproved(approved) = committed_event(&outcome) else {
            panic!("GateApproved を期待した");
        };
        assert_eq!(
            approved.phase_boundary(),
            Some(PhaseBoundary::new(PhaseId::Ideation, PhaseId::Inception))
        );
    }

    #[tokio::test]
    async fn approving_the_last_stage_reports_no_next_stage() {
        let mut subject = use_case(at_the_first_gate(2), 1);
        let outcome = subject
            .execute(&intent(), None, forward(), at())
            .await
            .expect("最終ステージも承認できる");
        let WorkflowExecutionEvent::GateApproved(approved) = committed_event(&outcome) else {
            panic!("GateApproved を期待した");
        };
        assert_eq!(approved.next_stage(), None);
        assert_eq!(approved.phase_boundary(), None);
    }

    // ---- 入力の正規化 ----

    #[test]
    fn every_reported_verdict_projects_onto_one_domain_verdict() {
        let cases = [
            (
                ReportedVerdict::Transition(ReportedTransition::AwaitingApproval {
                    artifacts: Vec::new(),
                }),
                Verdict::AwaitingApproval,
            ),
            (
                ReportedVerdict::Transition(ReportedTransition::Forward { user_input: None }),
                Verdict::Forward,
            ),
            (
                ReportedVerdict::Transition(ReportedTransition::Rejected { feedback: None }),
                Verdict::Rejected,
            ),
            (
                ReportedVerdict::Transition(ReportedTransition::Revised),
                Verdict::Revised,
            ),
            (
                ReportedVerdict::Transition(ReportedTransition::Skipped {
                    reason: "x".to_string(),
                }),
                Verdict::Skipped,
            ),
            (ReportedVerdict::Resumed, Verdict::Resume),
        ];
        for (reported, verdict) in cases {
            assert_eq!(reported.verdict(), verdict);
        }
    }
}
