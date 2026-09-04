//! 構造化リードモデルを引く 12 DAO の契約 — **1 表を鍵で引き、当たらなければ空**。
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
//! # 同じ契約を 2 実装に課す
//!
//! 各契約は**ジェネリック関数 1 本**として書き、SQLite 実装 (`*DaoImpl`) と in-memory
//! ダブル (`InMemory*Dao`) の**両方**から同一に呼ぶ (`coding-rules/good-examples.md`
//! §契約テスト)。ダブルの行は期待値の書き下しではなく、**同じフィクスチャを SQLite 実装で
//! 読み出して写したもの**である (`support::doubles`) — 両実装が同じ入力を見ていることが
//! 契約の前提だからである。ダブルが鍵を見ずに握った答えを返す形では、約束 3 も
//! 「行が無いこと自体が答え」(無効 scope・配信の終端) も表せない。
//!
//! # 契約の外 (実装ごとに違ってよいところ)
//!
//! 約束 4 の**起点**は実装で違う。SQLite 実装は `open` の時点で媒体を掴むので開けなければ
//! そこで失敗し、ダブルは媒体を持たないので `failing` で組んだときだけ失敗する。
//! したがって「引けない」の作り方は実装固有テストで固定し、契約関数には入れない。
//!
//! [`ReadModelReadError`]: core_query_use_case::orchestration::ReadModelReadError

// テストコードでは unwrap / expect / 添字を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

mod support;

use std::io::ErrorKind;

use core_query_interface_adapter::{
    InMemoryDefinitionDao, InMemoryExecutionDao, InMemoryJumpDao, InMemoryJumpPhaseDao,
    InMemoryNextAnswerDao, InMemoryPhaseEntryDao, InMemoryRunStageDao, InMemoryScopeChangeDao,
    InMemoryScopeDao, InMemoryScopeKeywordDao, InMemorySteeringPartDao, InMemorySteeringPlanDao,
    ReadModelDaos,
};
use core_query_use_case::orchestration::{
    DefinitionDao, ExecutionDao, JumpDao, JumpPhaseDao, NextAnswerDao, PhaseEntryDao,
    ReadModelReadError, RunStageDao, ScopeChangeDao, ScopeDao, ScopeKeywordDao, SteeringPartDao,
    SteeringPlanDao,
};

use support::{DEFINITION, EXECUTION, Fixture, INTENT, doubles};

/// 1 要求ぶんの読取専用接続を開き、12 実装をその上に建てる。
///
/// b44 で実装ごとの `open` は廃止された — 開く口は [`ReadModelDaos`] 1 か所で、12 実装は
/// その 1 接続を分け合う (多段の引当が同じスナップショットを見るため)。
fn daos(store: &std::path::Path) -> ReadModelDaos {
    ReadModelDaos::open(store).expect("フィクスチャのストアは開ける")
}

/// 投影されていない実行の識別子 (どの鍵にも当たらない)。
const ABSENT_EXECUTION: &str = "0190ffff-0000-7000-8000-000000000000";
/// 取り込まれていない定義の識別子。
const ABSENT_DEFINITION: &str = "kiro";

/// 束縛の桁は揃うが値がずれた 64 桁 (鍵の残余条件を外すための材料)。
fn wrong_digest() -> String {
    "0".repeat(64)
}

/// 内容ダイジェストの形 (64 桁の小文字 16 進)。
fn is_content_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
}

// ---------------------------------------------------------------------------
// NextAnswerDao — `read_next_answer` 1 表
// ---------------------------------------------------------------------------

/// 稼働中の実行に対する `next` の答え。
fn contract_next_answer<D: NextAnswerDao>(dao: &D) {
    let bare = dao.find(EXECUTION, "bare").unwrap().unwrap();
    assert_eq!(bare.decision_kind(), "run-stage");
    assert_eq!(bare.stage_index(), Some(1), "計画上の位置は行が運ぶ");
    assert_eq!(
        bare.stage_slug(),
        Some("intent-capture"),
        "state-init のゲートを開けた実行の次の一手 (答えは書込側の集約が決めている)"
    );
    assert_eq!(bare.gated(), Some(true));
    assert_eq!(bare.checkbox(), None, "まだ着手していないので印は無い");
    assert_eq!(
        bare.execution_id(),
        EXECUTION,
        "実行の面は結合せず FK 列だけを運ぶ"
    );
    assert!(
        bare.run_stage_id().is_some_and(is_content_digest),
        "run-stage の答えには材料を指す FK が在る"
    );

    // `--resume` の答えは再開メニューでステージを名指さない。
    let resume = dao.find(EXECUTION, "resume").unwrap().unwrap();
    assert_eq!(resume.decision_kind(), "resume-menu");
    assert_eq!(resume.stage_index(), None);
    assert_eq!(resume.stage_slug(), None);
    assert_eq!(resume.gated(), None);
    assert_eq!(resume.run_stage_id(), None);

    // 自由記述は新しい仕事の振り分けであって、いまの実行のステージではない。
    assert_eq!(
        dao.find(EXECUTION, "free-text")
            .unwrap()
            .unwrap()
            .decision_kind(),
        "new-work-routing"
    );

    assert_eq!(dao.find(ABSENT_EXECUTION, "bare").unwrap(), None);
    assert_eq!(dao.find(EXECUTION, "nonsense").unwrap(), None);
}

/// park 中の実行に対する `next` の答え (FK が NULL なら材料は無い)。
fn contract_next_answer_parked<D: NextAnswerDao>(dao: &D) {
    let parked = dao.find(EXECUTION, "bare").unwrap().unwrap();
    assert_eq!(parked.decision_kind(), "parked");
    assert_eq!(
        parked.stage_slug(),
        Some("intent-capture"),
        "park の答えは位置を名乗る"
    );
    assert_eq!(parked.gated(), None, "止まっているのでゲートの別が無い");
    assert_eq!(
        parked.run_stage_id(),
        None,
        "RMU は park の答えに材料の FK を書かない — 名指す slug から結合し直してはならない"
    );

    assert_eq!(
        dao.find(EXECUTION, "resume")
            .unwrap()
            .unwrap()
            .decision_kind(),
        "unpark-then-resume"
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
fn the_next_answer_contract_holds_on_the_store() {
    let fixture = Fixture::projected();
    contract_next_answer(&daos(fixture.store()).next_answer());
}

#[test]
fn the_next_answer_contract_holds_on_the_double() {
    let fixture = Fixture::projected();
    contract_next_answer(&doubles::next_answer(&fixture));
}

#[test]
fn the_parked_next_answer_contract_holds_on_the_store() {
    let fixture = Fixture::parked();
    contract_next_answer_parked(&daos(fixture.store()).next_answer());
}

#[test]
fn the_parked_next_answer_contract_holds_on_the_double() {
    let fixture = Fixture::parked();
    contract_next_answer_parked(&doubles::next_answer(&fixture));
}

// ---------------------------------------------------------------------------
// ExecutionDao — `read_execution` 1 表 (識別子 / 状態束縛の 2 鍵)
// ---------------------------------------------------------------------------

/// 稼働中の実行の現在地 (2 つの鍵は同じ 1 行に当たる)。
fn contract_execution<D: ExecutionDao>(dao: &D) {
    let found = dao.find(EXECUTION).unwrap().unwrap();
    assert_eq!(found.execution_id(), EXECUTION);
    assert_eq!(
        found.intent_id(),
        INTENT,
        "定義識別子は載せない — intent を指す FK だけを運ぶ"
    );
    assert_eq!(found.scope(), "classic");
    assert_eq!(found.status(), "running");
    assert_eq!(
        found.cursor_slug(),
        Some("intent-capture"),
        "ゲートを開けた先が現在地"
    );
    assert_eq!(found.parked_at_slug(), None, "止まっていない");
    assert!(!found.parked_active());
    assert!(is_content_digest(found.state_binding()));

    let by_binding = dao
        .find_by_state_binding(found.state_binding())
        .unwrap()
        .unwrap();
    assert_eq!(by_binding, found, "2 つの鍵は同じ 1 行を指す");

    assert_eq!(dao.find(ABSENT_EXECUTION).unwrap(), None);
    assert_eq!(
        dao.find_by_state_binding(&wrong_digest()).unwrap(),
        None,
        "束縛がずれれば当たらない"
    );
}

/// park 中の実行の現在地 (止まった位置を行が運ぶ)。
fn contract_execution_parked<D: ExecutionDao>(dao: &D) {
    let found = dao.find(EXECUTION).unwrap().unwrap();
    assert!(found.parked_active());
    assert_eq!(
        found.parked_at_slug(),
        Some("intent-capture"),
        "止まった位置は現在地とは別の列で運ぶ"
    );
    assert_eq!(found.cursor_slug(), found.parked_at_slug());
    assert_eq!(
        found.status(),
        "running",
        "park は実行の状態ではなく重ねた印である"
    );
}

#[test]
fn the_execution_contract_holds_on_the_store() {
    let fixture = Fixture::projected();
    contract_execution(&daos(fixture.store()).execution());
}

#[test]
fn the_execution_contract_holds_on_the_double() {
    let fixture = Fixture::projected();
    contract_execution(&doubles::execution(&fixture));
}

#[test]
fn the_parked_execution_contract_holds_on_the_store() {
    let fixture = Fixture::parked();
    contract_execution_parked(&daos(fixture.store()).execution());
}

#[test]
fn the_parked_execution_contract_holds_on_the_double() {
    let fixture = Fixture::parked();
    contract_execution_parked(&doubles::execution(&fixture));
}

// ---------------------------------------------------------------------------
// RunStageDao — `read_run_stage` 1 表 (自然キー / 代理キー / 束縛付きの 3 鍵)
// ---------------------------------------------------------------------------

/// run-stage の材料 23 列と、3 つの鍵の当たり方。
fn contract_run_stage<D: RunStageDao>(dao: &D) {
    // 任意フィールドまで埋めたステージ — 23 列を 1 つずつ突き合わせる。
    let found = dao
        .find(DEFINITION, "classic", "intent-capture")
        .unwrap()
        .unwrap();
    assert!(is_content_digest(found.id()), "行の写しは主キーを運ぶ");
    assert_eq!(found.definition_id(), DEFINITION);
    assert_eq!(found.scope(), "classic");
    assert_eq!(found.stage_slug(), "intent-capture");
    assert_eq!(found.phase(), "ideation");
    assert!(
        is_content_digest(found.steering_plan_id()),
        "配信計画をたどる FK を運ぶ"
    );
    assert_eq!(found.lead_agent(), "aidlc-product-agent");
    assert_eq!(
        found.support_agents(),
        r#"["aidlc-design-agent"]"#,
        "配列の列は 1 行 JSON の文字列のまま運ぶ (開くのは描く側)"
    );
    assert_eq!(found.mode(), "mob");
    assert!(found.gate_default());
    assert_eq!(
        found.inline_context_paths_rel(),
        r#"["agents/aidlc-product-agent.md"]"#
    );
    assert_eq!(found.stage_file_rel(), "ideation/intent-capture.md");
    assert_eq!(found.memory_path_rel(), "ideation/intent-capture/memory.md");
    assert_eq!(found.consumes_rel(), "[]");
    assert_eq!(
        found.produces_rel(),
        r#"["ideation/intent-capture/intent.md"]"#,
        "成果物は record からの相対で運ぶ"
    );
    assert_eq!(found.sensors_applicable(), "[]");
    assert_eq!(found.reviewer(), Some("aidlc-product-lead-agent"));
    assert_eq!(found.reviewer_max_iterations(), Some(2));
    assert_eq!(found.review_class(), Some("adversarial"));
    assert_eq!(found.protocol_modules(), r#"["reviewer","ensemble"]"#);
    assert_eq!(
        found.next_stage_name(),
        None,
        "classic は次を SKIP するので末尾扱い"
    );
    assert!(is_content_digest(found.route_digest()));
    assert!(is_content_digest(found.directive_digest()));

    // 任意フィールドを持たないステージ — 同じ列が空側の値で載る。
    let bare = dao
        .find(DEFINITION, "classic", "state-init")
        .unwrap()
        .unwrap();
    assert_eq!(bare.lead_agent(), "");
    assert_eq!(bare.support_agents(), "[]");
    assert_eq!(bare.mode(), "inline");
    assert!(!bare.gate_default());
    assert_eq!(bare.reviewer(), None);
    assert_eq!(bare.reviewer_max_iterations(), None);
    assert_eq!(bare.review_class(), None);
    assert_eq!(
        bare.next_stage_name(),
        Some("Intent Capture"),
        "実行される次が在れば表示名を運ぶ"
    );

    // 代理キー — 自然キーで引いた行と同じ 1 行に当たる (FK がたどれる)。
    assert_eq!(
        dao.find_by_id(found.id()).unwrap(),
        Some(found.clone()),
        "FK がたどれる — 自然キーで引いた行と同じ 1 行に当たる"
    );

    // 束縛は鍵の残余条件 — 1 つでもずれれば当たらない。
    assert_eq!(
        dao.find_bound(
            DEFINITION,
            "classic",
            "intent-capture",
            found.route_digest(),
            found.directive_digest(),
        )
        .unwrap(),
        Some(found.clone())
    );
    assert_eq!(
        dao.find_bound(
            DEFINITION,
            "classic",
            "intent-capture",
            &wrong_digest(),
            found.directive_digest(),
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
            found.route_digest(),
            &wrong_digest(),
        )
        .unwrap(),
        None,
        "directive 束縛がずれれば当たらない"
    );

    assert_eq!(dao.find(DEFINITION, "classic", "gone").unwrap(), None);
    assert_eq!(
        dao.find(ABSENT_DEFINITION, "classic", "intent-capture")
            .unwrap(),
        None
    );
    assert_eq!(dao.find_by_id("no-such-row").unwrap(), None);
}

#[test]
fn the_run_stage_contract_holds_on_the_store() {
    let fixture = Fixture::projected();
    contract_run_stage(&daos(fixture.store()).run_stage());
}

#[test]
fn the_run_stage_contract_holds_on_the_double() {
    let fixture = Fixture::projected();
    contract_run_stage(&doubles::run_stage(&fixture));
}

// ---------------------------------------------------------------------------
// SteeringPlanDao / SteeringPartDao — steering の 2 表 (それぞれ 1 表)
// ---------------------------------------------------------------------------

/// 配信計画 1 行 (鍵は run-stage が運ぶ FK)。
fn contract_steering_plan<D: SteeringPlanDao>(dao: &D, plan_id: &str) {
    let found = dao.find(plan_id).unwrap().unwrap();
    assert_eq!(found.id(), plan_id);
    assert_eq!(found.phase(), "ideation");
    assert!(is_content_digest(found.bundle_digest()));
    assert_eq!(found.part_count(), 1, "この束は 1 部に収まる");
    assert_eq!(
        found.delivered_paths(),
        r#"["org.md","phases/ideation.md"]"#,
        "どのルールを配ったかは計画が名乗る (中身は部の行が持つ)"
    );

    assert_eq!(
        dao.find_bound(plan_id, found.bundle_digest()).unwrap(),
        Some(found.clone())
    );
    assert_eq!(
        dao.find_bound(plan_id, &wrong_digest()).unwrap(),
        None,
        "bundle 束縛がずれれば当たらない"
    );
    assert_eq!(dao.find("no-such-plan").unwrap(), None);
}

/// 配信の部 (終端は行の有無が表す)。
fn contract_steering_part<D: SteeringPartDao>(dao: &D, plan_id: &str) {
    let found = dao.find(plan_id, D::FIRST_PART).unwrap().unwrap();
    assert_eq!(found.steering_plan_id(), plan_id);
    assert_eq!(found.part_index(), 1);
    assert_eq!(found.phase(), "ideation");
    assert_eq!(
        found.rules_content(),
        r##"[{"path":"org.md","text":"# Org\n"},{"path":"phases/ideation.md","text":"# Ideation\n"}]"##,
        "部が運ぶのは配る規則の中身そのもの"
    );

    assert_eq!(
        dao.find(plan_id, D::FIRST_PART + 1).unwrap(),
        None,
        "終端は行の有無で表す"
    );
    assert_eq!(dao.find("no-such-plan", D::FIRST_PART).unwrap(), None);
}

#[test]
fn the_steering_plan_contract_holds_on_the_store() {
    let fixture = Fixture::projected();
    let plan_id = doubles::ideation_plan_id(&fixture);
    contract_steering_plan(&daos(fixture.store()).steering_plan(), &plan_id);
}

#[test]
fn the_steering_plan_contract_holds_on_the_double() {
    let fixture = Fixture::projected();
    let plan_id = doubles::ideation_plan_id(&fixture);
    contract_steering_plan(&doubles::steering_plan(&fixture), &plan_id);
}

#[test]
fn the_steering_part_contract_holds_on_the_store() {
    let fixture = Fixture::projected();
    let plan_id = doubles::ideation_plan_id(&fixture);
    contract_steering_part(&daos(fixture.store()).steering_part(), &plan_id);
}

#[test]
fn the_steering_part_contract_holds_on_the_double() {
    let fixture = Fixture::projected();
    let plan_id = doubles::ideation_plan_id(&fixture);
    contract_steering_part(&doubles::steering_part(&fixture), &plan_id);
}

// ---------------------------------------------------------------------------
// JumpDao / JumpPhaseDao — ジャンプの 2 表 (それぞれ 1 表)
// ---------------------------------------------------------------------------

/// ジャンプ先ごとの受理判定 (拒否も 1 つの答え)。
fn contract_jump<D: JumpDao>(dao: &D) {
    let redo = dao.find(EXECUTION, "intent-capture").unwrap().unwrap();
    assert_eq!(redo.target_index(), 1);
    assert_eq!(redo.target_slug(), "intent-capture");
    assert_eq!(redo.outcome(), "redo", "現在地への跳躍はやり直し");
    assert_eq!(redo.refusal(), None, "受理された答えに拒否理由は無い");

    let refused = dao.find(EXECUTION, "state-init").unwrap().unwrap();
    assert_eq!(refused.outcome(), "refused");
    assert_eq!(
        refused.refusal(),
        Some("invalid-target"),
        "拒否も行として在る — 跳べるかの計算はクエリ側に無い"
    );

    assert_eq!(
        dao.find_by_target(EXECUTION, redo.target_index()).unwrap(),
        Some(redo),
        "フェーズ表の目的地からたどる鍵 — 同じ 1 行に当たる"
    );
    assert_eq!(dao.find_by_target(EXECUTION, 99).unwrap(), None);
    assert_eq!(dao.find(EXECUTION, "gone").unwrap(), None);
    assert_eq!(dao.find(ABSENT_EXECUTION, "intent-capture").unwrap(), None);
}

/// フェーズごとのジャンプ目的地 (受理判定は別の表)。
fn contract_jump_phase<D: JumpPhaseDao>(dao: &D) {
    let ideation = dao.find(EXECUTION, "ideation").unwrap().unwrap();
    assert_eq!(ideation.target_slug(), Some("intent-capture"));
    assert_eq!(
        ideation.target_index(),
        1,
        "受理判定へたどる鍵をそのまま運ぶ (受理そのものは別の表が持つ)"
    );

    let initialization = dao.find(EXECUTION, "initialization").unwrap().unwrap();
    assert_eq!(initialization.target_slug(), Some("state-init"));
    assert_eq!(initialization.target_index(), 0);

    assert_eq!(
        dao.find(EXECUTION, "operation").unwrap(),
        None,
        "目的地を持たないフェーズには行が無い"
    );
    assert_eq!(dao.find(ABSENT_EXECUTION, "ideation").unwrap(), None);
}

#[test]
fn the_jump_contract_holds_on_the_store() {
    let fixture = Fixture::projected();
    contract_jump(&daos(fixture.store()).jump());
}

#[test]
fn the_jump_contract_holds_on_the_double() {
    let fixture = Fixture::projected();
    contract_jump(&doubles::jump(&fixture));
}

#[test]
fn the_jump_phase_contract_holds_on_the_store() {
    let fixture = Fixture::projected();
    contract_jump_phase(&daos(fixture.store()).jump_phase());
}

#[test]
fn the_jump_phase_contract_holds_on_the_double() {
    let fixture = Fixture::projected();
    contract_jump_phase(&doubles::jump_phase(&fixture));
}

// ---------------------------------------------------------------------------
// 残り 5 ポート
// ---------------------------------------------------------------------------

/// scope カタログ 1 列 (行が返ること自体が「有効な scope」の答え)。
fn contract_scope<D: ScopeDao>(dao: &D) {
    // グリッド列を持つ scope — 11 列すべてが埋まる。
    let classic = dao.find(DEFINITION, "classic").unwrap().unwrap();
    assert_eq!(classic.scope(), "classic");
    assert_eq!(classic.depth(), Some("standard"));
    assert_eq!(classic.keywords(), r#"["api"]"#);
    assert_eq!(classic.skeleton(), Some("off"));
    assert_eq!(classic.review_cap(), Some("adversarial"));
    assert!(classic.freeform_default());
    assert!(classic.has_grid_column());
    assert_eq!(
        classic.cost_total(),
        Some(3),
        "グリッド列があるのでコストが載る"
    );
    assert_eq!(classic.cost_execute(), Some(2), "EXECUTE は 3 段中 2 段");
    assert_eq!(classic.cost_gates(), Some(1), "ゲートを持つのは 1 段");
    assert_eq!(classic.cost_per_unit_stages(), Some(0));

    // グリッド列を持たない scope — コスト 4 列がまとめて空になる。
    let express = dao.find(DEFINITION, "express").unwrap().unwrap();
    assert_eq!(express.scope(), "express");
    assert_eq!(express.depth(), None);
    assert_eq!(express.keywords(), r#"["quick"]"#);
    assert_eq!(express.skeleton(), None);
    assert_eq!(express.review_cap(), None);
    assert!(!express.freeform_default());
    assert!(!express.has_grid_column());
    assert_eq!(express.cost_total(), None);
    assert_eq!(express.cost_execute(), None);
    assert_eq!(express.cost_gates(), None);
    assert_eq!(express.cost_per_unit_stages(), None);

    assert_eq!(
        dao.find(DEFINITION, "feature").unwrap().unwrap().keywords(),
        "[]",
        "キーワードを持たない scope も空配列を運ぶ"
    );

    let names: Vec<String> = dao
        .find_stock(DEFINITION)
        .unwrap()
        .iter()
        .map(|view| view.scope().to_string())
        .collect();
    assert_eq!(
        names,
        ["express", "classic", "feature"],
        "既製 3 scope は upstream の定数の順で並ぶ"
    );

    let catalog: Vec<String> = dao
        .find_all(DEFINITION)
        .unwrap()
        .iter()
        .map(|view| view.scope().to_string())
        .collect();
    assert_eq!(
        catalog,
        ["classic", "express", "feature"],
        "カタログ全列は綴り順に並ぶ (拒否文言が並べる順そのもの)"
    );

    assert_eq!(dao.find(DEFINITION, "nonsense").unwrap(), None);
    assert_eq!(dao.find(ABSENT_DEFINITION, "classic").unwrap(), None);
    assert!(dao.find_stock(ABSENT_DEFINITION).unwrap().is_empty());
    assert!(
        dao.find_all(ABSENT_DEFINITION).unwrap().is_empty(),
        "取り込まれていない定義にはカタログが無い"
    );
}

/// キーワードから scope 名 (1 列しか無いので View 型を立てない)。
fn contract_scope_keyword<D: ScopeKeywordDao>(dao: &D) {
    assert_eq!(
        dao.find(DEFINITION, "api").unwrap(),
        Some("classic".to_string())
    );
    assert_eq!(
        dao.find(DEFINITION, "quick").unwrap(),
        Some("express".to_string()),
        "語は scope ごとに割り当たる"
    );
    assert_eq!(dao.find(DEFINITION, "unclaimed").unwrap(), None);
    assert_eq!(dao.find(ABSENT_DEFINITION, "api").unwrap(), None);
}

/// 定義側のフェーズ入口 (state を持たない要求からも引ける)。
fn contract_phase_entry<D: PhaseEntryDao>(dao: &D) {
    assert_eq!(
        dao.find(DEFINITION, "classic", "initialization")
            .unwrap()
            .unwrap()
            .first_stage_slug(),
        "state-init"
    );
    assert_eq!(
        dao.find(DEFINITION, "classic", "ideation")
            .unwrap()
            .unwrap()
            .first_stage_slug(),
        "intent-capture"
    );
    assert_eq!(
        dao.find(DEFINITION, "classic", "inception").unwrap(),
        None,
        "その scope で実行するステージが無いフェーズには行が無い"
    );
    assert_eq!(
        dao.find(DEFINITION, "express", "ideation").unwrap(),
        None,
        "グリッド列を持たない scope には入口が無い"
    );
    assert_eq!(
        dao.find(ABSENT_DEFINITION, "classic", "ideation").unwrap(),
        None
    );
}

/// 要求 scope と state の scope の照合結果。
fn contract_scope_change<D: ScopeChangeDao>(dao: &D) {
    assert_eq!(
        dao.find(EXECUTION, "classic").unwrap().unwrap().kind(),
        "same-as-state"
    );
    assert_eq!(
        dao.find(EXECUTION, "express").unwrap().unwrap().kind(),
        "scope-change",
        "state と同じ scope と違う scope で綴りが分かれる"
    );
    assert_eq!(
        dao.find(EXECUTION, "nonsense").unwrap(),
        None,
        "有効でない scope には行が無い"
    );
    assert_eq!(dao.find(ABSENT_EXECUTION, "classic").unwrap(), None);
}

/// 定義 1 行の要約 (引けないこと自体が「未取込」の答え)。
fn contract_definition<D: DefinitionDao>(dao: &D) {
    let found = dao.find(DEFINITION).unwrap().unwrap();
    assert_eq!(found.stage_count(), 3);
    assert_eq!(found.scope_count(), 3);
    assert!(found.revision().starts_with("sha256:"));
    assert_eq!(
        dao.find(ABSENT_DEFINITION).unwrap(),
        None,
        "取り込まれていない定義は行が無い"
    );
}

#[test]
fn the_scope_contract_holds_on_the_store() {
    let fixture = Fixture::projected();
    contract_scope(&daos(fixture.store()).scope());
}

#[test]
fn the_scope_contract_holds_on_the_double() {
    let fixture = Fixture::projected();
    contract_scope(&doubles::scope(&fixture));
}

#[test]
fn the_scope_keyword_contract_holds_on_the_store() {
    let fixture = Fixture::projected();
    contract_scope_keyword(&daos(fixture.store()).scope_keyword());
}

#[test]
fn the_scope_keyword_contract_holds_on_the_double() {
    let fixture = Fixture::projected();
    contract_scope_keyword(&doubles::scope_keyword(&fixture));
}

#[test]
fn the_phase_entry_contract_holds_on_the_store() {
    let fixture = Fixture::projected();
    contract_phase_entry(&daos(fixture.store()).phase_entry());
}

#[test]
fn the_phase_entry_contract_holds_on_the_double() {
    let fixture = Fixture::projected();
    contract_phase_entry(&doubles::phase_entry(&fixture));
}

#[test]
fn the_scope_change_contract_holds_on_the_store() {
    let fixture = Fixture::projected();
    contract_scope_change(&daos(fixture.store()).scope_change());
}

#[test]
fn the_scope_change_contract_holds_on_the_double() {
    let fixture = Fixture::projected();
    contract_scope_change(&doubles::scope_change(&fixture));
}

#[test]
fn the_definition_contract_holds_on_the_store() {
    let fixture = Fixture::projected();
    contract_definition(&daos(fixture.store()).definition());
}

#[test]
fn the_definition_contract_holds_on_the_double() {
    let fixture = Fixture::projected();
    contract_definition(&doubles::definition(&fixture));
}

// ---------------------------------------------------------------------------
// 読めない媒体 (契約の外 — 失敗の起点は実装で違う)
// ---------------------------------------------------------------------------

/// SQLite の接続は開くだけでは中身を読まない。開けたのに引けない媒体は、行の不在ではなく
/// **読取失敗**として上がる (1 行を引く口も、0 行以上を引く口も同じ)。
#[test]
fn a_store_that_opens_but_is_not_a_database_is_a_read_failure_on_every_lookup() {
    let dir = tempfile::tempdir().unwrap();
    let garbage = dir.path().join("garbage.sqlite3");
    std::fs::write(&garbage, b"not a sqlite database at all").unwrap();
    let daos = ReadModelDaos::open(&garbage).expect("開くだけなら通る");

    let one = daos
        .definition()
        .find(DEFINITION)
        .expect_err("1 行を引く口も潰える");
    assert_eq!(one.path(), Some(garbage.as_path()));

    let many = daos
        .scope()
        .find_all(DEFINITION)
        .expect_err("0 行以上を引く口も潰える");
    assert_eq!(many.path(), Some(garbage.as_path()));
    assert_eq!(many.kind(), one.kind(), "分類は媒体側の 1 本に収束する");
}

#[test]
fn a_store_that_is_not_there_is_a_read_failure_not_an_absent_row() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("no-such-store.sqlite3");
    let error = ReadModelDaos::open(&missing).expect_err("開けない");
    assert_eq!(error.path(), Some(missing.as_path()));
}

#[test]
fn a_failing_double_reports_the_read_failure_from_every_verb() {
    let error = ReadModelReadError::new(ErrorKind::WouldBlock, None);
    let digest = wrong_digest();

    assert_eq!(
        InMemoryNextAnswerDao::failing(error.clone()).find(EXECUTION, "bare"),
        Err(error.clone())
    );
    let execution = InMemoryExecutionDao::failing(error.clone());
    assert_eq!(execution.find(EXECUTION), Err(error.clone()));
    assert_eq!(execution.find_by_state_binding(&digest), Err(error.clone()));
    let run_stage = InMemoryRunStageDao::failing(error.clone());
    assert_eq!(
        run_stage.find(DEFINITION, "classic", "intent-capture"),
        Err(error.clone())
    );
    assert_eq!(run_stage.find_by_id("any"), Err(error.clone()));
    assert_eq!(
        run_stage.find_bound(DEFINITION, "classic", "intent-capture", &digest, &digest),
        Err(error.clone())
    );
    let plan = InMemorySteeringPlanDao::failing(error.clone());
    assert_eq!(plan.find("any"), Err(error.clone()));
    assert_eq!(plan.find_bound("any", &digest), Err(error.clone()));
    assert_eq!(
        InMemorySteeringPartDao::failing(error.clone()).find("any", 1),
        Err(error.clone())
    );
    let jump = InMemoryJumpDao::failing(error.clone());
    assert_eq!(jump.find(EXECUTION, "intent-capture"), Err(error.clone()));
    assert_eq!(jump.find_by_target(EXECUTION, 1), Err(error.clone()));
    assert_eq!(
        InMemoryJumpPhaseDao::failing(error.clone()).find(EXECUTION, "ideation"),
        Err(error.clone())
    );
    let scope = InMemoryScopeDao::failing(error.clone());
    assert_eq!(scope.find(DEFINITION, "classic"), Err(error.clone()));
    assert_eq!(scope.find_all(DEFINITION), Err(error.clone()));
    assert_eq!(
        scope.find_stock(DEFINITION),
        Err(error.clone()),
        "既定実装の 3 回の引当も 1 回目の失敗で潰える"
    );
    assert_eq!(
        InMemoryScopeKeywordDao::failing(error.clone()).find(DEFINITION, "api"),
        Err(error.clone())
    );
    assert_eq!(
        InMemoryPhaseEntryDao::failing(error.clone()).find(DEFINITION, "classic", "ideation"),
        Err(error.clone())
    );
    assert_eq!(
        InMemoryScopeChangeDao::failing(error.clone()).find(EXECUTION, "classic"),
        Err(error.clone())
    );
    assert_eq!(
        InMemoryDefinitionDao::failing(error.clone()).find(DEFINITION),
        Err(error)
    );
}

#[test]
fn an_empty_double_answers_absent_rather_than_failing() {
    assert_eq!(
        InMemoryNextAnswerDao::empty()
            .find(EXECUTION, "bare")
            .unwrap(),
        None,
        "行を 1 つも持たないダブルは「読めない」ではなく「行が無い」を答える"
    );
    assert_eq!(
        InMemoryScopeDao::empty().find_stock(DEFINITION).unwrap(),
        Vec::new()
    );
    assert_eq!(
        InMemoryScopeDao::empty().find_all(DEFINITION).unwrap(),
        Vec::new()
    );
}
