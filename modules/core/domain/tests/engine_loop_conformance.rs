//! ITF 準拠テスト (ADR 0003 決定 5) — `formal/orchestration/engine_loop.qnt` のトレースを
//! `WorkflowExecution` に再生し、全ステップで状態射影とディレクティブ観測を突き合わせる。
//! フィクスチャは `tests/conformance/fixtures/engine_loop/` にコミット済み (#meta 正規化済み)。
//! トレースの各遷移は `lastAction` で駆動する (lastAction 規約)。

// テストコードでは unwrap を許可 (オーナー規約)。integration test のヘルパは
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used)]

use core_domain::orchestration::{
    AutonomyMode, EngineSignal, PlanAction, Status, WorkflowExecution,
};
use core_domain::workspace::CheckboxState;
use serde_json::Value;

fn bigint(v: &Value) -> i64 {
    v["#bigint"].as_str().unwrap().parse().unwrap()
}

fn tag(v: &Value) -> &str {
    v["tag"].as_str().unwrap()
}

/// `int -> X` の #map を stage 順の Vec へ。
fn map_to_vec<T>(v: &Value, n: usize, f: impl Fn(&Value) -> T) -> Vec<T> {
    let pairs = v["#map"].as_array().unwrap();
    let mut out: Vec<Option<T>> = (0..n).map(|_| None).collect();
    for p in pairs {
        let (k, val) = (&p[0], &p[1]);
        out[usize::try_from(bigint(k)).unwrap()] = Some(f(val));
    }
    out.into_iter().map(Option::unwrap).collect()
}

fn checkbox_of(v: &Value) -> CheckboxState {
    match tag(v) {
        "Pending" => CheckboxState::Pending,
        "InProgress" => CheckboxState::InProgress,
        "AwaitingApproval" => CheckboxState::AwaitingApproval,
        "Revising" => CheckboxState::Revising,
        "CompletedBox" => CheckboxState::Completed,
        "SkippedBox" => CheckboxState::Skipped,
        t => panic!("unknown checkbox tag {t}"),
    }
}

fn plan_of(v: &Value) -> PlanAction {
    match tag(v) {
        "Execute" => PlanAction::Execute,
        "SkipPlan" => PlanAction::Skip,
        t => panic!("unknown plan tag {t}"),
    }
}

struct ModelState {
    last_action: String,
    directive_tag: String,
    directive_stage: Option<usize>,
    cursor: usize,
    status: String,
    parked_at: i64,
    autonomous: bool,
    plan: Vec<PlanAction>,
    overlay: Vec<PlanAction>,
    conditional: Vec<bool>,
    checkbox: Vec<CheckboxState>,
    approved: Vec<bool>,
}

fn parse_state(v: &Value) -> ModelState {
    let n = v["plan"]["#map"].as_array().unwrap().len();
    let d = &v["lastDirective"];
    let directive_tag = tag(d).to_string();
    let directive_stage =
        (directive_tag == "DRunStage").then(|| usize::try_from(bigint(&d["value"])).unwrap());
    ModelState {
        last_action: v["lastAction"].as_str().unwrap().to_string(),
        directive_tag,
        directive_stage,
        cursor: usize::try_from(bigint(&v["cursor"])).unwrap(),
        status: tag(&v["status"]).to_string(),
        parked_at: bigint(&v["parkedAt"]),
        autonomous: v["autonomous"].as_bool().unwrap(),
        plan: map_to_vec(&v["plan"], n, plan_of),
        overlay: map_to_vec(&v["overlay"], n, plan_of),
        conditional: map_to_vec(&v["conditional"], n, |b| b.as_bool().unwrap()),
        checkbox: map_to_vec(&v["checkbox"], n, checkbox_of),
        approved: map_to_vec(&v["approved"], n, |b| b.as_bool().unwrap()),
    }
}

fn assert_projection(agg: &WorkflowExecution, m: &ModelState, step: usize) {
    let n = agg.stage_count();
    assert_eq!(n, m.plan.len(), "step {step}: stage count");
    for s in 0..n {
        assert_eq!(agg.checkbox(s), m.checkbox[s], "step {step}: checkbox[{s}]");
        assert_eq!(
            agg.effective_plan(s),
            m.overlay[s],
            "step {step}: overlay[{s}]"
        );
        assert_eq!(agg.approved(s), m.approved[s], "step {step}: approved[{s}]");
    }
    assert_eq!(agg.cursor(), m.cursor, "step {step}: cursor");
    assert_eq!(
        agg.autonomy().is_autonomous(),
        m.autonomous,
        "step {step}: autonomy"
    );
    let parked = agg.parked_at().map_or(-1, |p| i64::try_from(p).unwrap());
    assert_eq!(parked, m.parked_at, "step {step}: parkedAt");
    match m.status.as_str() {
        "Running" => {
            assert_eq!(agg.status(), Status::Running, "step {step}: status");
            assert!(!agg.parked_active(), "step {step}: not parked");
        }
        "WorkflowParked" => {
            assert!(agg.parked_active(), "step {step}: parked active");
        }
        "WorkflowCompleted" => {
            assert_eq!(agg.status(), Status::Completed, "step {step}: completed");
        }
        s => panic!("unknown status {s}"),
    }
}

fn assert_signal(sig: EngineSignal, m: &ModelState, step: usize) {
    match (sig, m.directive_tag.as_str()) {
        (EngineSignal::RunStage(s), "DRunStage") => {
            assert_eq!(Some(s), m.directive_stage, "step {step}: run-stage target");
        }
        (EngineSignal::Done, "DDone")
        | (EngineSignal::Parked, "DParked")
        | (EngineSignal::EngineError, "DError") => {}
        (got, want) => panic!("step {step}: signal {got:?} vs model {want}"),
    }
}

fn replay(path: &std::path::Path, seen: &mut std::collections::BTreeSet<String>) {
    let text = std::fs::read_to_string(path).unwrap();
    let trace: Value = serde_json::from_str(&text).unwrap();
    let states: Vec<ModelState> = trace["states"]
        .as_array()
        .unwrap()
        .iter()
        .map(parse_state)
        .collect();
    let m0 = &states[0];
    assert_eq!(m0.last_action, "init");
    let mut agg = WorkflowExecution::start(m0.plan.clone(), m0.conditional.clone()).unwrap();
    assert_projection(&agg, m0, 0);

    for (i, m) in states.iter().enumerate().skip(1) {
        seen.insert(m.last_action.clone());
        let prev = &states[i - 1];
        match m.last_action.as_str() {
            // 観測アクション (状態不変)
            "next" | "next_parked" | "done_stutter" => {
                let sig = agg.next();
                assert_signal(sig, m, i);
            }
            "report_stale" => {
                // モデルは nondet に stale 対象を選ぶ — 前状態から有効な対象を 1 つ選んで
                // メンバーシップ検査 (frame 等価は assert_projection が担う)
                let s = (0..prev.cursor)
                    .find(|&s| prev.checkbox[s] == CheckboxState::Completed)
                    .unwrap();
                let sig = agg.stale_report(s).unwrap();
                assert_signal(sig, m, i);
            }
            // 遷移コマンド
            "report_forward" => {
                let sig = agg.report_forward().unwrap();
                assert_signal(sig, m, i);
            }
            "report_awaiting_approval" => agg.gate_start().unwrap(),
            "report_rejected" => agg.reject().unwrap(),
            "report_revised" => agg.revise().unwrap(),
            "report_skipped" => {
                let sig = agg.report_skipped().unwrap();
                assert_signal(sig, m, i);
            }
            "jump_forward" | "jump_backward" => {
                agg.jump(m.cursor).unwrap();
            }
            "jump_redo" => {
                agg.jump(prev.cursor).unwrap();
            }
            "park" => {
                let sig = agg.park().unwrap();
                assert_signal(sig, m, i);
            }
            "unpark" => agg.unpark().unwrap(),
            "recompose" => {
                let s = (0..prev.overlay.len())
                    .find(|&s| prev.overlay[s] != m.overlay[s])
                    .unwrap();
                agg.recompose_flip(s).unwrap();
            }
            "set_autonomy" => {
                let mode = if m.autonomous {
                    AutonomyMode::Autonomous
                } else {
                    AutonomyMode::Gated
                };
                agg.set_autonomy(mode).unwrap();
            }
            a => panic!("step {i}: unknown action {a}"),
        }
        assert_projection(&agg, m, i);
    }
}

#[test]
fn workflow_execution_conforms_to_every_committed_engine_loop_trace() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/conformance/fixtures/engine_loop");
    let mut count = 0;
    let mut seen = std::collections::BTreeSet::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "json") {
            replay(&path, &mut seen);
            count += 1;
        }
    }
    assert!(count >= 6, "expected committed fixtures, found {count}");
    // アクション網羅: 全アクションが少なくとも 1 つのコミット済みトレースに現れること。
    // 初回 6 シードの探索は report_revised / report_skipped を一度も踏んでおらず、該当
    // アームは fixture に対して死文だった。負形式インライン不変条件
    // (--invariant 'not(lastAction == "...")') で採取した trace-0x101 / trace-0x202 で
    // 補完済み — 稀アクションを含む fixture の消失退行をここで防ぐ。
    for action in [
        "next",
        "next_parked",
        "done_stutter",
        "report_stale",
        "report_forward",
        "report_awaiting_approval",
        "report_rejected",
        "report_revised",
        "report_skipped",
        "jump_forward",
        "jump_backward",
        "jump_redo",
        "park",
        "unpark",
        "recompose",
        "set_autonomy",
    ] {
        assert!(
            seen.contains(action),
            "no committed trace exercises action {action}"
        );
    }
}
