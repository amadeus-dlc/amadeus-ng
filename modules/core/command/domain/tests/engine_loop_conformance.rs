//! ITF 準拠テスト (ADR 0003 決定 5) — `formal/orchestration/engine_loop.qnt` のトレースを
//! イベントソーシング形の `IntentExecution` に **decide → apply** 経路で再生し、全ステップで
//! 状態射影を突き合わせる (BR2.5)。
//! フィクスチャは `tests/conformance/fixtures/engine_loop/` にコミット済み (#meta 正規化済み)。
//! トレースの各遷移は `lastAction` で駆動する (lastAction 規約)。
//!
//! # 遷移面と観測面の分割 (b26 段階 2)
//!
//! 本ファイルが担うのは**遷移面**である — 各アクションが集約の状態をモデルどおりに動かすこと
//! (`assert_projection` の frame 等価)。モデルの `lastDirective` が表す**観測面**
//! (「次に何をせよと言うか」) はここでは照合しない: directive を出すのは読むだけの動詞であり
//! クエリ側の責務なので (`coding-rules/cqrs-boundaries.md` 規則 5〜7)、その ITF は
//! `core-query-use-case/tests/engine_loop_ladder_conformance.rs` が同じフィクスチャで担う。
//! **アクション網羅のアサートは両ファイルで維持する** — 分割で片側の網羅が緩まないように。
//!
//! モデルの `gated(s) = s != 0` は **initialization フェーズ 1 ステージだけを持つ合成計画**への
//! 抽象である。ここではその合成計画 (索引 0 = initialization、以降 = inception) を
//! `Intent::create` へ直接与え、その対の左を `IntentExecution::start` に渡して集約を作る。実グラフの initialization が
//! 3 ステージであることは、集約側のユニットテスト (`gated = phase != initialization`) が固定する。

// テストコードでは unwrap を許可 (オーナー規約)。integration test のヘルパは
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
// indexing_slicing も同じ理由 (固定長フィクスチャの添字参照) で file 単位の allow が要る。
// panic! は想定外ケースの即時失敗という検証用途で使っており、テスト失敗のシグナルとして
// 妥当なため同様に許容する。
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    AutonomyMode, Created, EngineSignal, Intent, IntentEventId, IntentExecution, IntentExecutionId,
    IntentId, NextRequest, StageDisplay, StageEntry, StageIndex, StartRequest, Status,
    WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
    WorkflowDefinitionId,
};
use core_command_domain::workspace::CheckboxState;
use serde_json::Value;

/// ITF 再生は時計を持たない — `occurred_at` は固定値でよい (集約は値を素通しする)。
const AT_TEXT: &str = "2026-08-23T00:00:00Z";

/// 固定の発生時刻。
fn at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(AT_TEXT)
        .unwrap()
        .with_timezone(&Utc)
}

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
    /// `DRunStage` の対象ステージ (観測面の照合に使う)。
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

fn slug(index: usize) -> StageSlug {
    StageSlug::parse(&format!("stage-{index}")).unwrap()
}

fn synthetic_id() -> WorkflowDefinitionId {
    WorkflowDefinitionId::parse("itf").unwrap()
}

fn synthetic_revision() -> DefinitionRevision {
    DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap()
}

/// モデルの初期 plan / conditional から合成計画を作る (索引 0 = initialization)。
fn synthetic_stages(m: &ModelState) -> Vec<StageEntry> {
    m.plan
        .iter()
        .zip(m.conditional.iter())
        .enumerate()
        .map(|(index, (action, conditional))| {
            let phase = if index == 0 {
                PhaseId::Initialization
            } else {
                PhaseId::Inception
            };
            StageEntry::new(
                slug(index),
                phase,
                *action,
                *conditional,
                display(&format!("{}.{}", phase.index(), index + 1)),
            )
        })
        .collect()
}

/// ITF 再生の表示属性 (モデルは表示を持たないので固定でよい — 投影の検収は別テスト)。
fn display(number: &str) -> StageDisplay {
    StageDisplay::new(StageNumber::parse(number).unwrap(), "Stage", "orchestrator").unwrap()
}

/// ITF 再生の走査結果 (同上)。
fn scan() -> WorkspaceScan {
    WorkspaceScan::new(
        BrownfieldGreenfield::Greenfield,
        "Unknown",
        "Unknown",
        "Unknown",
    )
    .unwrap()
}

fn index(agg: &IntentExecution, value: usize) -> StageIndex {
    agg.stage_index(value).unwrap()
}

fn assert_projection(agg: &IntentExecution, m: &ModelState, step: usize) {
    let n = agg.stage_count();
    assert_eq!(n, m.plan.len(), "step {step}: stage count");
    for s in 0..n {
        let stage = index(agg, s);
        assert_eq!(
            agg.checkbox(stage),
            Some(m.checkbox[s]),
            "step {step}: checkbox[{s}]"
        );
        assert_eq!(
            agg.effective_plan(stage),
            Some(m.overlay[s]),
            "step {step}: overlay[{s}]"
        );
        assert_eq!(
            agg.approved(stage),
            Some(m.approved[s]),
            "step {step}: approved[{s}]"
        );
    }
    assert_eq!(agg.cursor().to_usize(), m.cursor, "step {step}: cursor");
    assert_eq!(
        agg.autonomy().is_autonomous(),
        m.autonomous,
        "step {step}: autonomy"
    );
    let parked = agg
        .parked_at()
        .map_or(-1, |p| i64::try_from(p.to_usize()).unwrap());
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

/// 観測面 — 集約の判断 (`next_decision`) をモデルの `lastDirective` と突き合わせる (BR3.1)。
///
/// 判断は集約が所有する (仕様 10 §2.3。2026-09-02 の裁定でクエリ側から復帰)。RMU は
/// この同じクエリを呼んでリードモデルへ投影するので、ここで固定した対応がそのまま
/// `read_next_answer` の正しさの根拠になる。
fn assert_signal(agg: &IntentExecution, m: &ModelState, step: usize) {
    let decision = agg.next_decision(&NextRequest::default());
    let signal = EngineSignal::from(&decision);
    match (signal, m.directive_tag.as_str()) {
        (EngineSignal::RunStage(s), "DRunStage") => {
            assert_eq!(
                Some(s.to_usize()),
                m.directive_stage,
                "step {step}: run-stage target"
            );
        }
        (EngineSignal::Done, "DDone")
        | (EngineSignal::Parked, "DParked")
        | (EngineSignal::EngineError, "DError") => {}
        (got, want) => panic!("step {step}: signal {got:?} vs model {want}"),
    }
}

/// 遷移コマンドの後にモデルが載せるディレクティブ (report のエピローグ / park の停止) を照合する。
///
/// これはモデル側フィクスチャの健全性チェックである (遷移コマンドの直後の `lastDirective` は
/// 遷移の帰結であって、集約の観測クエリの答えではない)。
fn assert_directive(m: &ModelState, want: &str, step: usize) {
    assert_eq!(
        m.directive_tag, want,
        "step {step}: directive after {}",
        m.last_action
    );
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
    // 合成計画からの組み直しは完全コンストラクタ経由の再構成を通す — イベントは不要で、
    // 検査点は genesis と同一である (coding-rules/aggregate-commands.md)。
    let intent = Intent::from((
        Created::new(
            IntentEventId::generate(),
            IntentId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
            synthetic_id(),
            synthetic_revision(),
            StartRequest::new("itf", "conformance"),
            synthetic_stages(m0),
            scan(),
        ),
        at(),
    ));
    let (mut agg, _started) = IntentExecution::start(
        IntentExecutionId::parse("018f3b2c-4d5e-7f60-8abc-def012345678").unwrap(),
        &intent,
        at(),
    );
    assert_eq!(agg.seq_nr(), 1, "genesis の通番は 1 (BR2.1)");
    assert_projection(&agg, m0, 0);

    for (i, m) in states.iter().enumerate().skip(1) {
        seen.insert(m.last_action.clone());
        let prev = &states[i - 1];
        match m.last_action.as_str() {
            // 観測アクション (状態不変)。集約の判断をモデルの directive と突き合わせる
            // (観測面)。frame 等価 (観測は状態を動かさない) は末尾の assert_projection が担う。
            "next" | "next_parked" | "done_stutter" => assert_signal(&agg, m, i),
            "report_stale" => {
                // モデルは nondet に stale 対象を選ぶ — 前状態から有効な対象を 1 つ選んで
                // メンバーシップ検査 (frame 等価は assert_projection が担う)。ガードは受理可否
                // だけを答え、何もコミットしない (BR1.9)。
                let s = (0..prev.cursor)
                    .find(|&s| prev.checkbox[s] == CheckboxState::Completed)
                    .unwrap();
                agg.stale_report(index(&agg, s)).unwrap();
            }
            // 遷移コマンド (decide → 1 イベント → apply)
            "report_forward" => {
                // 前進はゲート承認だけである (BR1.3)。誕生が initialization を完了済みにする
                // (b34) ので、カーソルは常にゲート付きステージに立つ — 非ゲート完了の
                // コマンドは b42 で撤去した (#85 = A)。
                agg.approve_gate(&intent, None, at()).unwrap();
                assert_directive(m, "DDone", i);
            }
            "report_awaiting_approval" => {
                agg.open_gate(&intent, Vec::new(), at()).unwrap();
            }
            "report_rejected" => {
                agg.reject_gate(&intent, None, at()).unwrap();
            }
            "report_revised" => {
                agg.revise_stage(&intent, at()).unwrap();
            }
            "report_skipped" => {
                agg.skip_stage(&intent, "conformance".to_string(), at())
                    .unwrap();
                assert_directive(m, "DDone", i);
            }
            "jump_forward" | "jump_backward" => {
                let target = index(&agg, m.cursor);
                agg.jump(&intent, target, at()).unwrap();
            }
            "jump_redo" => {
                let target = index(&agg, prev.cursor);
                agg.jump(&intent, target, at()).unwrap();
            }
            "park" => {
                // 再スタンプ (park 済みへの park) はモデルでも `lastAction == "park"` なので、
                // 合成アクション名 `repark` を立てて網羅アサートの対象にする — この経路を
                // 含むフィクスチャが失われたら赤くなる。
                if agg.parked_active() {
                    seen.insert("repark".to_string());
                }
                agg.park(&intent, at()).unwrap();
                assert_directive(m, "DParked", i);
            }
            "unpark" => {
                agg.unpark(&intent, at()).unwrap();
            }
            "recompose" => {
                // モデルの actRecompose は 1 ステージ反転 — 要素数 1 の recompose に対応 (BR2.5)。
                let s = (0..prev.overlay.len())
                    .find(|&s| prev.overlay[s] != m.overlay[s])
                    .unwrap();
                agg.recompose(&intent, &[index(&agg, s)], at()).unwrap();
            }
            "set_autonomy" => {
                // モデルはトグル — 反転後の値を switch_autonomy に渡す (BR2.5)。
                let mode = if m.autonomous {
                    AutonomyMode::Autonomous
                } else {
                    AutonomyMode::Gated
                };
                agg.switch_autonomy(&intent, mode, at()).unwrap();
            }
            a => panic!("step {i}: unknown action {a}"),
        }
        assert_projection(&agg, m, i);
    }
}

#[test]
fn intent_conforms_to_every_committed_engine_loop_trace() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../tests/conformance/fixtures/engine_loop");
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
        // 合成アクション — park 済みへの park (再スタンプ)。trace-0x303 が持つ。
        "repark",
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
