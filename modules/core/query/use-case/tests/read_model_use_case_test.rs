//! 構造化リードモデルを引く 12 ユースケースの契約 — **鍵を渡し、FK をたどって組む**。
//!
//! クエリ側のユースケースに許されるのは引当だけである
//! (`coding-rules/cqrs-boundaries.md` 規則 6)。DAO は 1 表しか引かないので、複数の表に
//! またがる答え (`next` の 1 ターン・`continue` の続き・フェーズジャンプ) は
//! **ユースケースが FK をたどって表ごとに引き**、組み立て View を返す
//! (オーナー裁定 2026-09-03)。したがってここが固定するのは 4 つである:
//!
//! 1. 渡した鍵がそのまま DAO へ届くこと (鍵を作り替えない)
//! 2. 次の引当の鍵が**前の行の FK 列そのまま**であること (自然キーで引き直さない)
//! 3. FK が NULL なら「無し」で伝播すること — 行の値を見て段を増やさない
//! 4. DAO が返した答え (行・不在・失敗) がそのまま戻ること (畳まない・言い換えない)
//!
//! フェイクは「合鍵のときだけ行を返す」形にしてある — 鍵が届いていなければ `None` に
//! なるので、1 と 2 を 1 本のアサーションで見られる。

// テストコードでは unwrap / expect を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::ErrorKind;
use std::path::PathBuf;

use core_query_use_case::orchestration::{
    DefinitionDao, DefinitionSummaryView, ExecutionDao, ExecutionView, FindContinuationUseCase,
    FindDefinitionUseCase, FindExecutionUseCase, FindJumpUseCase, FindNextAnswerUseCase,
    FindPhaseEntryUseCase, FindRunStageUseCase, FindScopeChangeUseCase, FindScopeKeywordUseCase,
    FindScopeUseCase, JumpDao, JumpPhaseDao, JumpPhaseView, JumpView, NextAnswerDao,
    NextAnswerView, PhaseEntryDao, PhaseEntryView, ReadModelReadError, RunStageDao, RunStageView,
    ScopeChangeDao, ScopeChangeView, ScopeDao, ScopeKeywordDao, ScopeView, SteeringPartDao,
    SteeringPartView, SteeringPlanDao, SteeringPlanView,
};

const DEFINITION: &str = "claude";
const EXECUTION: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000";
const RUN_STAGE: &str = "run-stage-row-id";
const PLAN: &str = "steering-plan-row-id";
const STATE_BINDING: &str = "state-binding";
const ROUTE: &str = "route-digest";
const DIRECTIVE: &str = "directive-digest";
const BUNDLE: &str = "bundle-digest";

fn failure() -> ReadModelReadError {
    ReadModelReadError::new(
        ErrorKind::WouldBlock,
        Some(PathBuf::from("/r/store.sqlite3")),
    )
}

fn run_stage() -> RunStageView {
    RunStageView::new(
        RUN_STAGE.to_string(),
        DEFINITION.to_string(),
        "classic".to_string(),
        "intent-capture".to_string(),
        "ideation".to_string(),
        PLAN.to_string(),
        "aidlc-product-agent".to_string(),
        "[]".to_string(),
        "inline".to_string(),
        true,
        "[]".to_string(),
        "ideation/intent-capture.md".to_string(),
        "ideation/intent-capture/memory.md".to_string(),
        "[]".to_string(),
        "[]".to_string(),
        "[]".to_string(),
        None,
        None,
        None,
        "[]".to_string(),
        None,
        ROUTE.to_string(),
        DIRECTIVE.to_string(),
    )
}

fn execution() -> ExecutionView {
    ExecutionView::new(
        EXECUTION.to_string(),
        "intent".to_string(),
        "classic".to_string(),
        "running".to_string(),
        Some("intent-capture".to_string()),
        None,
        false,
        STATE_BINDING.to_string(),
    )
}

fn plan() -> SteeringPlanView {
    SteeringPlanView::new(
        PLAN.to_string(),
        "ideation".to_string(),
        BUNDLE.to_string(),
        3,
        r#"["org.md"]"#.to_string(),
    )
}

fn part(part_index: u32) -> SteeringPartView {
    SteeringPartView::new(
        PLAN.to_string(),
        "ideation".to_string(),
        part_index,
        "[]".to_string(),
    )
}

// ---------------------------------------------------------------------------
// フェイク — 合鍵のときだけ行を返す (鍵が届いていなければ `None` になる)
// ---------------------------------------------------------------------------

/// `read_next_answer` の 1 行を握る (`run_stage_id` はテストが決める)。
struct FakeNextAnswerDao {
    run_stage_id: Option<String>,
}

impl NextAnswerDao for FakeNextAnswerDao {
    fn find(
        &self,
        execution_id: &str,
        request_kind: &str,
    ) -> Result<Option<NextAnswerView>, ReadModelReadError> {
        if (execution_id, request_kind) != (EXECUTION, "bare") {
            return Ok(None);
        }
        Ok(Some(NextAnswerView::new(
            "run-stage".to_string(),
            Some(1),
            Some("intent-capture".to_string()),
            Some(true),
            None,
            EXECUTION.to_string(),
            self.run_stage_id.clone(),
        )))
    }
}

struct FailingNextAnswerDao;

impl NextAnswerDao for FailingNextAnswerDao {
    fn find(&self, _: &str, _: &str) -> Result<Option<NextAnswerView>, ReadModelReadError> {
        Err(failure())
    }
}

/// 実行の面。`present` が偽なら行が無い (同一スナップショットの FK が宙に浮いた形)。
struct FakeExecutionDao {
    present: bool,
}

impl FakeExecutionDao {
    const fn new(present: bool) -> FakeExecutionDao {
        FakeExecutionDao { present }
    }
}

impl ExecutionDao for FakeExecutionDao {
    fn find(&self, execution_id: &str) -> Result<Option<ExecutionView>, ReadModelReadError> {
        if !self.present || execution_id != EXECUTION {
            return Ok(None);
        }
        Ok(Some(execution()))
    }

    fn find_by_state_binding(
        &self,
        state_binding: &str,
    ) -> Result<Option<ExecutionView>, ReadModelReadError> {
        if !self.present || state_binding != STATE_BINDING {
            return Ok(None);
        }
        Ok(Some(execution()))
    }
}

/// run-stage の面。3 動詞とも同じ 1 行を握る (鍵は動詞ごとの形で照合する)。
struct FakeRunStageDao {
    present: bool,
}

impl FakeRunStageDao {
    const fn new(present: bool) -> FakeRunStageDao {
        FakeRunStageDao { present }
    }
}

impl RunStageDao for FakeRunStageDao {
    fn find(
        &self,
        definition_id: &str,
        scope: &str,
        stage_slug: &str,
    ) -> Result<Option<RunStageView>, ReadModelReadError> {
        if (definition_id, scope, stage_slug) != (DEFINITION, "classic", "intent-capture") {
            return Ok(None);
        }
        Ok(Some(run_stage()))
    }

    fn find_by_id(&self, id: &str) -> Result<Option<RunStageView>, ReadModelReadError> {
        if !self.present || id != RUN_STAGE {
            return Ok(None);
        }
        Ok(Some(run_stage()))
    }

    fn find_bound(
        &self,
        definition_id: &str,
        scope: &str,
        stage_slug: &str,
        route_digest: &str,
        directive_digest: &str,
    ) -> Result<Option<RunStageView>, ReadModelReadError> {
        let key = (
            definition_id,
            scope,
            stage_slug,
            route_digest,
            directive_digest,
        );
        if !self.present || key != (DEFINITION, "classic", "intent-capture", ROUTE, DIRECTIVE) {
            return Ok(None);
        }
        Ok(Some(run_stage()))
    }
}

/// 配信計画の面。`present` が偽なら未パック (別トランザクションなので不在は正常)。
struct FakeSteeringPlanDao {
    present: bool,
}

impl FakeSteeringPlanDao {
    const fn new(present: bool) -> FakeSteeringPlanDao {
        FakeSteeringPlanDao { present }
    }
}

impl SteeringPlanDao for FakeSteeringPlanDao {
    fn find(&self, id: &str) -> Result<Option<SteeringPlanView>, ReadModelReadError> {
        if !self.present || id != PLAN {
            return Ok(None);
        }
        Ok(Some(plan()))
    }

    fn find_bound(
        &self,
        id: &str,
        bundle_digest: &str,
    ) -> Result<Option<SteeringPlanView>, ReadModelReadError> {
        if !self.present || (id, bundle_digest) != (PLAN, BUNDLE) {
            return Ok(None);
        }
        Ok(Some(plan()))
    }
}

/// 配信の 1 部。`held` の番号だけが行を持つ。
struct FakeSteeringPartDao {
    held: Option<u32>,
}

impl FakeSteeringPartDao {
    const fn new(held: Option<u32>) -> FakeSteeringPartDao {
        FakeSteeringPartDao { held }
    }
}

impl SteeringPartDao for FakeSteeringPartDao {
    fn find(
        &self,
        steering_plan_id: &str,
        part_index: u32,
    ) -> Result<Option<SteeringPartView>, ReadModelReadError> {
        if steering_plan_id != PLAN || self.held != Some(part_index) {
            return Ok(None);
        }
        Ok(Some(part(part_index)))
    }
}

// ---------------------------------------------------------------------------
// FindNextAnswerUseCase — 答え → 実行 → run-stage → 計画 → 1 部目
// ---------------------------------------------------------------------------

#[test]
fn the_next_turn_follows_the_foreign_keys_of_the_answer_row() {
    let answers = FakeNextAnswerDao {
        run_stage_id: Some(RUN_STAGE.to_string()),
    };
    let use_case = FindNextAnswerUseCase::new(
        answers,
        FakeExecutionDao::new(true),
        FakeRunStageDao::new(true),
        FakeSteeringPlanDao::new(true),
        FakeSteeringPartDao::new(Some(1)),
    );

    let found = use_case.execute(EXECUTION, "bare").unwrap().unwrap();
    assert_eq!(found.answer().decision_kind(), "run-stage");
    assert_eq!(found.execution().scope(), "classic");
    assert_eq!(
        found.run_stage().map(RunStageView::stage_slug),
        Some("intent-capture"),
        "答えの run_stage_id をそのまま鍵にして引く"
    );
    assert_eq!(
        found.plan().map(SteeringPlanView::bundle_digest),
        Some(BUNDLE),
        "run-stage の steering_plan_id をそのまま鍵にして引く"
    );
    assert_eq!(
        found.first_part().map(SteeringPartView::part_index),
        Some(1),
        "1 部目はポート定数が決める — SQL のリテラルではない"
    );

    assert_eq!(
        use_case.execute(EXECUTION, "resume").unwrap(),
        None,
        "起点の鍵が違えば引当は空 — ユースケースは代わりの答えを作らない"
    );
}

#[test]
fn a_parked_answer_carries_no_run_stage_even_though_it_names_a_stage() {
    // 実データの再現: park の答えは `stage_slug` を持つが `run_stage_id` は NULL である。
    // 自然キー (`definition_id` × `scope` × `stage_slug`) で引き直すと材料が付いてしまうので、
    // フェイクはその鍵に対して**行を持っている**状態にしてある。
    let use_case = FindNextAnswerUseCase::new(
        FakeNextAnswerDao { run_stage_id: None },
        FakeExecutionDao::new(true),
        FakeRunStageDao::new(true),
        FakeSteeringPlanDao::new(true),
        FakeSteeringPartDao::new(Some(1)),
    );

    let found = use_case.execute(EXECUTION, "bare").unwrap().unwrap();
    assert_eq!(
        found.run_stage(),
        None,
        "FK が NULL なら材料は無い — RMU が「無し」と書いた関連を作らない"
    );
    assert_eq!(found.plan(), None, "たどる起点が無いので計画も無い");
    assert_eq!(found.first_part(), None, "1 部目も無い");
}

#[test]
fn the_next_turn_stops_at_the_run_stage_when_the_steering_is_not_packed_yet() {
    let use_case = FindNextAnswerUseCase::new(
        FakeNextAnswerDao {
            run_stage_id: Some(RUN_STAGE.to_string()),
        },
        FakeExecutionDao::new(true),
        FakeRunStageDao::new(true),
        FakeSteeringPlanDao::new(false),
        FakeSteeringPartDao::new(Some(1)),
    );

    let found = use_case.execute(EXECUTION, "bare").unwrap().unwrap();
    assert!(found.run_stage().is_some());
    assert_eq!(
        found.plan(),
        None,
        "steering の 2 表は別トランザクションで差し替わる — 不在は正常な観測"
    );
    assert_eq!(found.first_part(), None);
}

#[test]
fn an_empty_plan_leaves_the_first_part_absent() {
    let use_case = FindNextAnswerUseCase::new(
        FakeNextAnswerDao {
            run_stage_id: Some(RUN_STAGE.to_string()),
        },
        FakeExecutionDao::new(true),
        FakeRunStageDao::new(true),
        FakeSteeringPlanDao::new(true),
        FakeSteeringPartDao::new(None),
    );

    let found = use_case.execute(EXECUTION, "bare").unwrap().unwrap();
    assert!(found.plan().is_some());
    assert_eq!(found.first_part(), None, "空計画は行の有無で表す");
}

#[test]
fn a_dangling_execution_reference_is_a_broken_projection() {
    let use_case = FindNextAnswerUseCase::new(
        FakeNextAnswerDao {
            run_stage_id: Some(RUN_STAGE.to_string()),
        },
        FakeExecutionDao::new(false),
        FakeRunStageDao::new(true),
        FakeSteeringPlanDao::new(true),
        FakeSteeringPartDao::new(Some(1)),
    );

    let error = use_case.execute(EXECUTION, "bare").expect_err("壊れた投影");
    assert_eq!(error.kind(), ErrorKind::InvalidData);
}

#[test]
fn a_dangling_run_stage_reference_is_a_broken_projection() {
    let use_case = FindNextAnswerUseCase::new(
        FakeNextAnswerDao {
            run_stage_id: Some(RUN_STAGE.to_string()),
        },
        FakeExecutionDao::new(true),
        FakeRunStageDao::new(false),
        FakeSteeringPlanDao::new(true),
        FakeSteeringPartDao::new(Some(1)),
    );

    let error = use_case.execute(EXECUTION, "bare").expect_err("壊れた投影");
    assert_eq!(
        error.kind(),
        ErrorKind::InvalidData,
        "同じスナップショットの FK が指す先を引けないのは不在ではなく壊れた投影である"
    );
}

#[test]
fn the_next_answer_use_case_surfaces_the_read_failure_unchanged() {
    let use_case = FindNextAnswerUseCase::new(
        FailingNextAnswerDao,
        FakeExecutionDao::new(true),
        FakeRunStageDao::new(true),
        FakeSteeringPlanDao::new(true),
        FakeSteeringPartDao::new(Some(1)),
    );
    assert_eq!(use_case.execute(EXECUTION, "bare"), Err(failure()));
}

// ---------------------------------------------------------------------------
// FindContinuationUseCase — run-stage → 計画 → 要求された部
// ---------------------------------------------------------------------------

const fn continuation(
    run_stages: FakeRunStageDao,
    plans: FakeSteeringPlanDao,
    parts: FakeSteeringPartDao,
) -> FindContinuationUseCase<FakeRunStageDao, FakeSteeringPlanDao, FakeSteeringPartDao> {
    FindContinuationUseCase::new(run_stages, plans, parts)
}

#[test]
fn the_continuation_takes_every_binding_as_part_of_a_key() {
    let use_case = continuation(
        FakeRunStageDao::new(true),
        FakeSteeringPlanDao::new(true),
        FakeSteeringPartDao::new(Some(2)),
    );

    let found = use_case
        .execute(
            DEFINITION,
            "classic",
            "intent-capture",
            ROUTE,
            DIRECTIVE,
            BUNDLE,
            2,
        )
        .unwrap()
        .unwrap();
    assert_eq!(found.run_stage().stage_slug(), "intent-capture");
    assert_eq!(found.plan().part_count(), 3);
    assert_eq!(found.next_part().map(SteeringPartView::part_index), Some(2));

    let wrong = "0".repeat(64);
    assert_eq!(
        use_case
            .execute(
                DEFINITION,
                "classic",
                "intent-capture",
                &wrong,
                DIRECTIVE,
                BUNDLE,
                2
            )
            .unwrap(),
        None,
        "route 束縛がずれれば当たらない"
    );
    assert_eq!(
        use_case
            .execute(
                DEFINITION,
                "classic",
                "intent-capture",
                ROUTE,
                DIRECTIVE,
                &wrong,
                2
            )
            .unwrap(),
        None,
        "bundle 束縛がずれれば当たらない"
    );
}

#[test]
fn the_continuation_is_empty_when_the_plan_does_not_match() {
    let use_case = continuation(
        FakeRunStageDao::new(true),
        FakeSteeringPlanDao::new(false),
        FakeSteeringPartDao::new(Some(2)),
    );
    assert_eq!(
        use_case
            .execute(
                DEFINITION,
                "classic",
                "intent-capture",
                ROUTE,
                DIRECTIVE,
                BUNDLE,
                2
            )
            .unwrap(),
        None,
        "計画に当たらない token は fail-closed — 続きを届けない"
    );
}

#[test]
fn a_part_index_past_the_last_part_leaves_the_next_part_empty() {
    let use_case = continuation(
        FakeRunStageDao::new(true),
        FakeSteeringPlanDao::new(true),
        FakeSteeringPartDao::new(Some(2)),
    );
    let found = use_case
        .execute(
            DEFINITION,
            "classic",
            "intent-capture",
            ROUTE,
            DIRECTIVE,
            BUNDLE,
            99,
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        found.next_part(),
        None,
        "終端は行の有無で表す — 計画そのものは引けたままである"
    );
}

// ---------------------------------------------------------------------------
// FindExecutionUseCase (識別子 / 状態束縛)
// ---------------------------------------------------------------------------

#[test]
fn the_execution_use_case_hands_both_keys_through() {
    let use_case = FindExecutionUseCase::new(FakeExecutionDao::new(true));
    assert_eq!(
        use_case.execute(EXECUTION).unwrap().unwrap().scope(),
        "classic"
    );
    assert_eq!(use_case.execute("other").unwrap(), None);
    assert_eq!(
        use_case
            .execute_by_state_binding(STATE_BINDING)
            .unwrap()
            .unwrap()
            .execution_id(),
        EXECUTION
    );
    assert_eq!(use_case.execute_by_state_binding("other").unwrap(), None);
}

// ---------------------------------------------------------------------------
// FindRunStageUseCase
// ---------------------------------------------------------------------------

#[test]
fn the_run_stage_use_case_hands_the_three_part_key_through() {
    let use_case = FindRunStageUseCase::new(FakeRunStageDao::new(true));
    assert_eq!(
        use_case
            .execute(DEFINITION, "classic", "intent-capture")
            .unwrap()
            .unwrap()
            .route_digest(),
        ROUTE
    );
    assert_eq!(
        use_case
            .execute(DEFINITION, "express", "intent-capture")
            .unwrap(),
        None
    );
}

// ---------------------------------------------------------------------------
// FindJumpUseCase — slug 指定は 1 引当、phase 指定はフェーズ表 → ジャンプ表の 2 引当
// ---------------------------------------------------------------------------

struct FakeJumpDao {
    by_target: Option<u32>,
}

impl JumpDao for FakeJumpDao {
    fn find(
        &self,
        execution_id: &str,
        target_slug: &str,
    ) -> Result<Option<JumpView>, ReadModelReadError> {
        if (execution_id, target_slug) != (EXECUTION, "intent-capture") {
            return Ok(None);
        }
        Ok(Some(JumpView::new(
            1,
            "intent-capture".to_string(),
            "backward".to_string(),
            None,
        )))
    }

    fn find_by_target(
        &self,
        execution_id: &str,
        target_index: u32,
    ) -> Result<Option<JumpView>, ReadModelReadError> {
        if execution_id != EXECUTION || self.by_target != Some(target_index) {
            return Ok(None);
        }
        Ok(Some(JumpView::new(
            target_index,
            "intent-capture".to_string(),
            "refused".to_string(),
            Some("not-running".to_string()),
        )))
    }
}

struct FakeJumpPhaseDao;

impl JumpPhaseDao for FakeJumpPhaseDao {
    fn find(
        &self,
        execution_id: &str,
        phase: &str,
    ) -> Result<Option<JumpPhaseView>, ReadModelReadError> {
        if (execution_id, phase) != (EXECUTION, "ideation") {
            return Ok(None);
        }
        Ok(Some(JumpPhaseView::new(
            1,
            Some("intent-capture".to_string()),
        )))
    }
}

#[test]
fn the_jump_use_case_hands_the_slug_key_through() {
    let use_case = FindJumpUseCase::new(FakeJumpDao { by_target: Some(1) }, FakeJumpPhaseDao);
    assert_eq!(
        use_case
            .execute(EXECUTION, "intent-capture")
            .unwrap()
            .unwrap()
            .outcome(),
        "backward"
    );
    assert_eq!(use_case.execute(EXECUTION, "gone").unwrap(), None);
}

#[test]
fn the_phase_jump_follows_the_target_index_into_the_outcome_table() {
    let use_case = FindJumpUseCase::new(FakeJumpDao { by_target: Some(1) }, FakeJumpPhaseDao);
    assert_eq!(
        use_case
            .execute_phase(EXECUTION, "ideation")
            .unwrap()
            .unwrap()
            .refusal(),
        Some("not-running"),
        "フェーズ表の目的地で受理判定を引き直す — 拒否も 1 つの答えである"
    );
    assert_eq!(
        use_case.execute_phase(EXECUTION, "operation").unwrap(),
        None,
        "目的地を持たないフェーズには行が無い"
    );
}

#[test]
fn a_phase_target_without_an_outcome_row_is_a_broken_projection() {
    let use_case = FindJumpUseCase::new(FakeJumpDao { by_target: None }, FakeJumpPhaseDao);
    let error = use_case
        .execute_phase(EXECUTION, "ideation")
        .expect_err("壊れた投影");
    assert_eq!(error.kind(), ErrorKind::InvalidData);
}

// ---------------------------------------------------------------------------
// ScopeDao (find / find_stock)
// ---------------------------------------------------------------------------

struct FakeScopeDao;

fn scope_view(scope: &str) -> ScopeView {
    ScopeView::new(
        scope.to_string(),
        Some("standard".to_string()),
        "[]".to_string(),
        None,
        None,
        false,
        true,
        Some(30),
        Some(20),
        Some(10),
        Some(4),
    )
}

impl ScopeDao for FakeScopeDao {
    /// 綴り順で並べた 2 列 (`feature` は [`FakeScopeDao::find`] が持たないので現れない)。
    fn find_all(&self, definition_id: &str) -> Result<Vec<ScopeView>, ReadModelReadError> {
        if definition_id != DEFINITION {
            return Ok(Vec::new());
        }
        Ok(["classic", "express"].into_iter().map(scope_view).collect())
    }

    fn find(
        &self,
        definition_id: &str,
        scope: &str,
    ) -> Result<Option<ScopeView>, ReadModelReadError> {
        if definition_id != DEFINITION || scope == "feature" {
            return Ok(None);
        }
        Ok(Some(scope_view(scope)))
    }
}

#[test]
fn the_scope_use_case_hands_the_key_through() {
    let use_case = FindScopeUseCase::new(FakeScopeDao);
    assert_eq!(
        use_case
            .execute(DEFINITION, "classic")
            .unwrap()
            .unwrap()
            .scope(),
        "classic"
    );
    assert_eq!(use_case.execute("other", "classic").unwrap(), None);
}

#[test]
fn the_stock_lookup_keeps_the_upstream_order_and_skips_what_is_missing() {
    let use_case = FindScopeUseCase::new(FakeScopeDao);
    let names: Vec<String> = use_case
        .execute_stock(DEFINITION)
        .unwrap()
        .iter()
        .map(|view| view.scope().to_string())
        .collect();
    assert_eq!(
        names,
        ["express", "classic"],
        "定数の順を保ち、引けなかった feature は落ちる"
    );
}

// ---------------------------------------------------------------------------
// ScopeKeywordDao
// ---------------------------------------------------------------------------

struct FakeScopeKeywordDao;

impl ScopeKeywordDao for FakeScopeKeywordDao {
    fn find(
        &self,
        definition_id: &str,
        keyword: &str,
    ) -> Result<Option<String>, ReadModelReadError> {
        if definition_id != DEFINITION || keyword != "bug" {
            return Ok(None);
        }
        Ok(Some("bugfix".to_string()))
    }
}

#[test]
fn the_scope_keyword_use_case_hands_the_key_through() {
    let use_case = FindScopeKeywordUseCase::new(FakeScopeKeywordDao);
    assert_eq!(
        use_case.execute(DEFINITION, "bug").unwrap(),
        Some("bugfix".to_string())
    );
    assert_eq!(use_case.execute(DEFINITION, "feature").unwrap(), None);
}

// ---------------------------------------------------------------------------
// PhaseEntryDao
// ---------------------------------------------------------------------------

struct FakePhaseEntryDao;

impl PhaseEntryDao for FakePhaseEntryDao {
    fn find(
        &self,
        definition_id: &str,
        scope: &str,
        phase: &str,
    ) -> Result<Option<PhaseEntryView>, ReadModelReadError> {
        if (definition_id, scope, phase) != (DEFINITION, "classic", "ideation") {
            return Ok(None);
        }
        Ok(Some(PhaseEntryView::new("intent-capture".to_string())))
    }
}

#[test]
fn the_phase_entry_use_case_hands_the_three_part_key_through() {
    let use_case = FindPhaseEntryUseCase::new(FakePhaseEntryDao);
    assert_eq!(
        use_case
            .execute(DEFINITION, "classic", "ideation")
            .unwrap()
            .unwrap()
            .first_stage_slug(),
        "intent-capture"
    );
    assert_eq!(
        use_case
            .execute(DEFINITION, "classic", "operation")
            .unwrap(),
        None
    );
}

// ---------------------------------------------------------------------------
// ScopeChangeDao
// ---------------------------------------------------------------------------

struct FakeScopeChangeDao;

impl ScopeChangeDao for FakeScopeChangeDao {
    fn find(
        &self,
        execution_id: &str,
        scope: &str,
    ) -> Result<Option<ScopeChangeView>, ReadModelReadError> {
        if (execution_id, scope) != (EXECUTION, "express") {
            return Ok(None);
        }
        Ok(Some(ScopeChangeView::new("scope-change".to_string())))
    }
}

#[test]
fn the_scope_change_use_case_hands_the_key_through() {
    let use_case = FindScopeChangeUseCase::new(FakeScopeChangeDao);
    assert_eq!(
        use_case
            .execute(EXECUTION, "express")
            .unwrap()
            .unwrap()
            .kind(),
        "scope-change"
    );
    assert_eq!(
        use_case.execute(EXECUTION, "nonsense").unwrap(),
        None,
        "有効でない scope には行が無い"
    );
}

// ---------------------------------------------------------------------------
// DefinitionDao
// ---------------------------------------------------------------------------

struct FakeDefinitionDao;

impl DefinitionDao for FakeDefinitionDao {
    fn find(
        &self,
        definition_id: &str,
    ) -> Result<Option<DefinitionSummaryView>, ReadModelReadError> {
        if definition_id != DEFINITION {
            return Ok(None);
        }
        Ok(Some(DefinitionSummaryView::new(
            "sha256:0".to_string(),
            33,
            9,
        )))
    }
}

#[test]
fn the_definition_use_case_hands_the_key_through() {
    let use_case = FindDefinitionUseCase::new(FakeDefinitionDao);
    assert_eq!(
        use_case.execute(DEFINITION).unwrap().unwrap().stage_count(),
        33
    );
    assert_eq!(
        use_case.execute("kiro").unwrap(),
        None,
        "取り込まれていない定義は行が無い"
    );
}
