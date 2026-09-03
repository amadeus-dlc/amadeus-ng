//! 構造化リードモデルを引く 12 DAO 実装の契約 — **1 表を鍵で引き、当たらなければ空**。
//!
//! 行は RMU 本体が書いたものである (`support::Fixture`)。したがってここが固定するのは
//! 実装の 4 つの約束だけである:
//!
//! 1. ポートが宣言した鍵でその行が引けること
//! 2. **引くのは 1 表だけ**であること — 返る View は行の写しで、他の表の列を持たない。
//!    関連行をたどるのはユースケースの仕事なので、View は `id` と FK 列を運ぶ
//!    (オーナー裁定 2026-09-03 — `coding-rules/cqrs-boundaries.md` 規則 6)
//! 3. 鍵が当たらなければ `Ok(None)` になること (代わりの答えを作らない)
//! 4. 読めない媒体は [`ReadModelReadError`] になること (不在と読取失敗を混ぜない)
//!
//! [`ReadModelReadError`]: core_query_use_case::orchestration::ReadModelReadError

// テストコードでは unwrap / expect / 添字を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

mod support;

use core_query_interface_adapter::{
    DefinitionDaoImpl, ExecutionDaoImpl, JumpDaoImpl, JumpPhaseDaoImpl, NextAnswerDaoImpl,
    PhaseEntryDaoImpl, RunStageDaoImpl, ScopeChangeDaoImpl, ScopeDaoImpl, ScopeKeywordDaoImpl,
    SteeringPartDaoImpl, SteeringPlanDaoImpl,
};
use core_query_use_case::orchestration::{
    DefinitionDao, ExecutionDao, JumpDao, JumpPhaseDao, NextAnswerDao, PhaseEntryDao, RunStageDao,
    ScopeChangeDao, ScopeDao, ScopeKeywordDao, SteeringPartDao, SteeringPlanDao,
};

use support::{DEFINITION, EXECUTION, Fixture};

// ---------------------------------------------------------------------------
// NextAnswerDao — `read_next_answer` 1 表
// ---------------------------------------------------------------------------

#[test]
fn the_next_answer_is_found_by_execution_and_request_kind() {
    let fixture = Fixture::projected();
    let dao = NextAnswerDaoImpl::open(fixture.store()).unwrap();

    let found = dao.find(EXECUTION, "bare").unwrap().unwrap();
    assert_eq!(found.decision_kind(), "run-stage");
    assert_eq!(
        found.stage_slug(),
        Some("intent-capture"),
        "state-init のゲートを開けた実行の次の一手 (答えは書込側の集約が決めている)"
    );
    assert_eq!(found.gated(), Some(true));
    assert_eq!(
        found.execution_id(),
        EXECUTION,
        "実行の面は結合せず FK 列だけを運ぶ"
    );
    assert!(
        found.run_stage_id().is_some(),
        "run-stage の答えには材料を指す FK が在る"
    );
}

#[test]
fn a_decision_that_names_no_stage_carries_no_run_stage_reference() {
    let fixture = Fixture::projected();
    let dao = NextAnswerDaoImpl::open(fixture.store()).unwrap();

    // `--resume` の答えは再開メニューでステージを名指さない。
    let found = dao.find(EXECUTION, "resume").unwrap().unwrap();
    assert_eq!(found.stage_slug(), None);
    assert_eq!(found.run_stage_id(), None);
}

#[test]
fn a_parked_answer_names_a_stage_but_carries_no_run_stage_reference() {
    let fixture = Fixture::parked();
    let dao = NextAnswerDaoImpl::open(fixture.store()).unwrap();

    let parked = dao.find(EXECUTION, "bare").unwrap().unwrap();
    assert_eq!(parked.decision_kind(), "parked");
    assert_eq!(
        parked.stage_slug(),
        Some("intent-capture"),
        "park の答えは位置を名乗る"
    );
    assert_eq!(
        parked.run_stage_id(),
        None,
        "RMU は park の答えに材料の FK を書かない — 名指す slug から結合し直してはならない"
    );

    let reentry = dao.find(EXECUTION, "reentry").unwrap().unwrap();
    assert_eq!(reentry.decision_kind(), "run-stage");
    assert_eq!(
        reentry.stage_slug(),
        parked.stage_slug(),
        "同じ位置を名乗る 2 行 — 違うのは決定であって slug ではない"
    );
    assert!(
        reentry.run_stage_id().is_some(),
        "再入の答えは run-stage なので材料の FK が在る"
    );
}

#[test]
fn the_next_answer_is_empty_when_the_key_does_not_match() {
    let fixture = Fixture::projected();
    let dao = NextAnswerDaoImpl::open(fixture.store()).unwrap();

    assert_eq!(
        dao.find("0190ffff-0000-7000-8000-000000000000", "bare")
            .unwrap(),
        None
    );
    assert_eq!(dao.find(EXECUTION, "nonsense").unwrap(), None);
}

// ---------------------------------------------------------------------------
// ExecutionDao — `read_execution` 1 表 (識別子 / 状態束縛の 2 鍵)
// ---------------------------------------------------------------------------

#[test]
fn the_execution_is_found_by_its_identifier() {
    let fixture = Fixture::projected();
    let dao = ExecutionDaoImpl::open(fixture.store()).unwrap();

    let found = dao.find(EXECUTION).unwrap().unwrap();
    assert_eq!(found.execution_id(), EXECUTION);
    assert_eq!(found.scope(), "classic");
    assert_eq!(found.status(), "running");
    assert!(!found.parked_active());
    assert!(
        !found.intent_id().is_empty(),
        "定義識別子は載せない — intent を指す FK だけを運ぶ"
    );
    assert_eq!(
        dao.find("0190ffff-0000-7000-8000-000000000000").unwrap(),
        None
    );
}

#[test]
fn the_execution_is_found_by_its_state_binding() {
    let fixture = Fixture::projected();
    let dao = ExecutionDaoImpl::open(fixture.store()).unwrap();
    let binding = dao.find(EXECUTION).unwrap().unwrap();

    let found = dao
        .find_by_state_binding(binding.state_binding())
        .unwrap()
        .unwrap();
    assert_eq!(found.execution_id(), EXECUTION);
    assert_eq!(
        dao.find_by_state_binding(&"0".repeat(64)).unwrap(),
        None,
        "束縛がずれれば当たらない"
    );
}

// ---------------------------------------------------------------------------
// RunStageDao — `read_run_stage` 1 表 (自然キー / 代理キー / 束縛付きの 3 鍵)
// ---------------------------------------------------------------------------

#[test]
fn the_run_stage_is_found_by_definition_scope_and_slug() {
    let fixture = Fixture::projected();
    let dao = RunStageDaoImpl::open(fixture.store()).unwrap();

    let found = dao
        .find(DEFINITION, "classic", "intent-capture")
        .unwrap()
        .unwrap();
    assert_eq!(found.phase(), "ideation");
    assert_eq!(found.lead_agent(), "aidlc-product-agent");
    assert_eq!(found.mode(), "mob");
    assert_eq!(found.reviewer(), Some("aidlc-product-lead-agent"));
    assert_eq!(found.reviewer_max_iterations(), Some(2));
    assert!(found.stage_file_rel().ends_with("intent-capture.md"));
    assert!(!found.id().is_empty(), "行の写しは主キーを運ぶ");
    assert!(
        !found.steering_plan_id().is_empty(),
        "配信計画をたどる FK を運ぶ"
    );
    assert_eq!(dao.find(DEFINITION, "classic", "gone").unwrap(), None);
}

#[test]
fn the_run_stage_is_found_by_its_identifier() {
    let fixture = Fixture::projected();
    let dao = RunStageDaoImpl::open(fixture.store()).unwrap();
    let by_natural_key = dao
        .find(DEFINITION, "classic", "intent-capture")
        .unwrap()
        .unwrap();

    assert_eq!(
        dao.find_by_id(by_natural_key.id()).unwrap(),
        Some(by_natural_key.clone()),
        "FK がたどれる — 自然キーで引いた行と同じ 1 行に当たる"
    );
    assert_eq!(dao.find_by_id("no-such-row").unwrap(), None);
}

#[test]
fn the_bound_run_stage_takes_the_two_digests_as_part_of_the_key() {
    let fixture = Fixture::projected();
    let dao = RunStageDaoImpl::open(fixture.store()).unwrap();
    let row = dao
        .find(DEFINITION, "classic", "intent-capture")
        .unwrap()
        .unwrap();
    let wrong = "0".repeat(64);

    assert_eq!(
        dao.find_bound(
            DEFINITION,
            "classic",
            "intent-capture",
            row.route_digest(),
            row.directive_digest(),
        )
        .unwrap(),
        Some(row.clone())
    );
    assert_eq!(
        dao.find_bound(
            DEFINITION,
            "classic",
            "intent-capture",
            &wrong,
            row.directive_digest(),
        )
        .unwrap(),
        None,
        "route 束縛がずれれば当たらない"
    );
    assert_eq!(
        dao.find_bound(
            DEFINITION,
            "classic",
            "intent-capture",
            row.route_digest(),
            &wrong,
        )
        .unwrap(),
        None,
        "directive 束縛がずれれば当たらない"
    );
}

// ---------------------------------------------------------------------------
// SteeringPlanDao / SteeringPartDao — steering の 2 表 (それぞれ 1 表)
// ---------------------------------------------------------------------------

#[test]
fn the_steering_plan_is_found_by_the_run_stage_foreign_key() {
    let fixture = Fixture::projected();
    let run_stage = RunStageDaoImpl::open(fixture.store())
        .unwrap()
        .find(DEFINITION, "classic", "intent-capture")
        .unwrap()
        .unwrap();
    let dao = SteeringPlanDaoImpl::open(fixture.store()).unwrap();

    let found = dao.find(run_stage.steering_plan_id()).unwrap().unwrap();
    assert_eq!(found.id(), run_stage.steering_plan_id());
    assert_eq!(found.phase(), "ideation");
    assert!(found.part_count() >= 1);
    assert!(!found.bundle_digest().is_empty());
    assert_eq!(dao.find("no-such-plan").unwrap(), None);
}

#[test]
fn the_bound_steering_plan_takes_the_bundle_digest_as_part_of_the_key() {
    let fixture = Fixture::projected();
    let run_stage = RunStageDaoImpl::open(fixture.store())
        .unwrap()
        .find(DEFINITION, "classic", "intent-capture")
        .unwrap()
        .unwrap();
    let dao = SteeringPlanDaoImpl::open(fixture.store()).unwrap();
    let plan = dao.find(run_stage.steering_plan_id()).unwrap().unwrap();

    assert_eq!(
        dao.find_bound(plan.id(), plan.bundle_digest())
            .unwrap()
            .as_ref(),
        Some(&plan)
    );
    assert_eq!(
        dao.find_bound(plan.id(), &"0".repeat(64)).unwrap(),
        None,
        "bundle 束縛がずれれば当たらない"
    );
}

#[test]
fn the_steering_part_is_found_by_the_plan_foreign_key_and_the_part_number() {
    let fixture = Fixture::projected();
    let run_stage = RunStageDaoImpl::open(fixture.store())
        .unwrap()
        .find(DEFINITION, "classic", "intent-capture")
        .unwrap()
        .unwrap();
    let plan = SteeringPlanDaoImpl::open(fixture.store())
        .unwrap()
        .find(run_stage.steering_plan_id())
        .unwrap()
        .unwrap();
    let dao = SteeringPartDaoImpl::open(fixture.store()).unwrap();

    let found = dao
        .find(plan.id(), SteeringPartDaoImpl::FIRST_PART)
        .unwrap()
        .unwrap();
    assert_eq!(found.steering_plan_id(), plan.id());
    assert_eq!(found.part_index(), 1);
    assert_eq!(found.phase(), "ideation");
    assert!(!found.rules_content().is_empty());
    assert_eq!(
        dao.find(plan.id(), 99).unwrap(),
        None,
        "終端は行の有無で表す"
    );
}

// ---------------------------------------------------------------------------
// JumpDao / JumpPhaseDao — ジャンプの 2 表 (それぞれ 1 表)
// ---------------------------------------------------------------------------

#[test]
fn the_jump_outcome_is_found_by_execution_and_target_slug() {
    let fixture = Fixture::projected();
    let dao = JumpDaoImpl::open(fixture.store()).unwrap();

    let found = dao.find(EXECUTION, "intent-capture").unwrap().unwrap();
    assert_eq!(found.target_slug(), "intent-capture");
    assert!(!found.outcome().is_empty());
    assert_eq!(dao.find(EXECUTION, "gone").unwrap(), None);
}

#[test]
fn the_jump_outcome_is_found_by_execution_and_target_index() {
    let fixture = Fixture::projected();
    let dao = JumpDaoImpl::open(fixture.store()).unwrap();
    let by_slug = dao.find(EXECUTION, "intent-capture").unwrap().unwrap();

    assert_eq!(
        dao.find_by_target(EXECUTION, by_slug.target_index())
            .unwrap(),
        Some(by_slug),
        "フェーズ表の目的地からたどる鍵 — 同じ 1 行に当たる"
    );
    assert_eq!(dao.find_by_target(EXECUTION, 99).unwrap(), None);
}

#[test]
fn the_phase_jump_target_is_found_by_execution_and_phase() {
    let fixture = Fixture::projected();
    let dao = JumpPhaseDaoImpl::open(fixture.store()).unwrap();

    let found = dao.find(EXECUTION, "ideation").unwrap().unwrap();
    assert_eq!(found.target_slug(), Some("intent-capture"));
    assert_eq!(
        found.target_index(),
        JumpDaoImpl::open(fixture.store())
            .unwrap()
            .find(EXECUTION, "intent-capture")
            .unwrap()
            .unwrap()
            .target_index(),
        "受理判定へたどる鍵をそのまま運ぶ (受理そのものは別の表が持つ)"
    );
    assert_eq!(dao.find(EXECUTION, "operation").unwrap(), None);
}

// ---------------------------------------------------------------------------
// 残り 5 ポート
// ---------------------------------------------------------------------------

#[test]
fn the_scope_row_is_found_by_definition_and_name() {
    let fixture = Fixture::projected();
    let dao = ScopeDaoImpl::open(fixture.store()).unwrap();

    let found = dao.find(DEFINITION, "classic").unwrap().unwrap();
    assert_eq!(found.depth(), Some("standard"));
    assert!(found.has_grid_column());
    assert!(
        found.cost_total().is_some(),
        "グリッド列があるのでコストが載る"
    );
    assert_eq!(dao.find(DEFINITION, "nonsense").unwrap(), None);
}

#[test]
fn the_stock_scopes_come_back_in_the_upstream_order() {
    let fixture = Fixture::projected();
    let dao = ScopeDaoImpl::open(fixture.store()).unwrap();

    let names: Vec<String> = dao
        .find_stock(DEFINITION)
        .unwrap()
        .iter()
        .map(|view| view.scope().to_string())
        .collect();
    assert_eq!(names, ["express", "classic", "feature"]);
}

#[test]
fn the_scope_of_a_keyword_is_found_by_definition_and_word() {
    let fixture = Fixture::projected();
    let dao = ScopeKeywordDaoImpl::open(fixture.store()).unwrap();

    assert_eq!(
        dao.find(DEFINITION, "api").unwrap(),
        Some("classic".to_string())
    );
    assert_eq!(dao.find(DEFINITION, "unclaimed").unwrap(), None);
}

#[test]
fn the_phase_entry_is_found_by_definition_scope_and_phase() {
    let fixture = Fixture::projected();
    let dao = PhaseEntryDaoImpl::open(fixture.store()).unwrap();

    assert_eq!(
        dao.find(DEFINITION, "classic", "ideation")
            .unwrap()
            .unwrap()
            .first_stage_slug(),
        "intent-capture"
    );
    assert_eq!(dao.find(DEFINITION, "classic", "operation").unwrap(), None);
}

#[test]
fn the_scope_change_verdict_is_found_by_execution_and_requested_scope() {
    let fixture = Fixture::projected();
    let dao = ScopeChangeDaoImpl::open(fixture.store()).unwrap();

    let same = dao.find(EXECUTION, "classic").unwrap().unwrap();
    let changed = dao.find(EXECUTION, "express").unwrap().unwrap();
    assert_ne!(
        same.kind(),
        changed.kind(),
        "state と同じ scope と違う scope で綴りが分かれる"
    );
    assert_eq!(
        dao.find(EXECUTION, "nonsense").unwrap(),
        None,
        "有効でない scope には行が無い"
    );
}

#[test]
fn the_definition_summary_is_found_by_its_identifier() {
    let fixture = Fixture::projected();
    let dao = DefinitionDaoImpl::open(fixture.store()).unwrap();

    let found = dao.find(DEFINITION).unwrap().unwrap();
    assert_eq!(found.stage_count(), 3);
    assert_eq!(found.scope_count(), 3);
    assert!(found.revision().starts_with("sha256:"));
    assert_eq!(
        dao.find("kiro").unwrap(),
        None,
        "取り込まれていない定義は行が無い"
    );
}

// ---------------------------------------------------------------------------
// 読めない媒体
// ---------------------------------------------------------------------------

#[test]
fn a_store_that_is_not_there_is_a_read_failure_not_an_absent_row() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("no-such-store.sqlite3");
    let error = NextAnswerDaoImpl::open(&missing).expect_err("開けない");
    assert_eq!(error.path(), Some(missing.as_path()));
}
