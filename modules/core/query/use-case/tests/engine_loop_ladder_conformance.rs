//! ITF 準拠テスト (ADR 0003 決定 5) — `formal/orchestration/engine_loop.qnt` の **directive
//! 観測面**の準拠。モデルが `lastDirective` に載せる 4 値と、リードモデルのビューが下す判断
//! ([`ExecutionStateView::next_decision`]) の射影 [`EngineSignal`] を突き合わせる (BR3.1)。
//!
//! # 遷移面との分割 (b26 段階 2)
//!
//! 同じフィクスチャ (`tests/conformance/fixtures/engine_loop/`) を 2 つのテストが読む。
//! **遷移面** — 各アクションが状態をモデルどおりに動かすこと — はコマンド側の
//! `core-command-domain/tests/engine_loop_conformance.rs` が集約の decide → apply 経路で担う。
//! 本ファイルが担うのは**観測面** — 「次に何をせよと言うか」— であり、directive を出すのは
//! 読むだけの動詞なのでクエリ側の責務である (`coding-rules/cqrs-boundaries.md` 規則 5〜7)。
//! **アクション網羅のアサートは両ファイルで維持する** — 分割で片側の網羅が緩まないように。
//!
//! # なぜ再生ではなく直接構築なのか
//!
//! クエリ側は集約を再構成しない (同規則 6)。読むのは RMU が投影したリードモデルだけで、
//! モデルの状態変数はそのリードモデルの内容と 1:1 に対応する。したがってここでは各ステップの
//! モデル状態から**ビューを直接組み立てて**判断を照合する。イベント再生を通さないぶん、
//! 「投影が正しく書けているか」ではなく「同じ観測から同じ判断が出るか」だけを見ている
//! — 投影そのものの検収は RMU 側のテストが担う。
//!
//! モデルの `gated(s) = s != 0` は **initialization フェーズ 1 ステージだけを持つ合成計画**への
//! 抽象である。ここではその合成計画 (索引 0 = initialization、以降 = inception) を
//! Stage Progress 行の `phase` として与える。

// テストコードでは unwrap を許可 (オーナー規約)。integration test のヘルパは
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
// indexing_slicing も同じ理由 (固定長フィクスチャの添字参照) で file 単位の allow が要る。
// panic! は想定外ケースの即時失敗という検証用途で使っており、テスト失敗のシグナルとして
// 妥当なため同様に許容する。
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use core_query_use_case::execution_view::{
    CheckboxState, ExecutionStateView, ExecutionStatus, StageProgressView,
};
use core_query_use_case::orchestration::{EngineSignal, NextRequest};
use core_query_use_case::workflow_view::{PhaseView, PlanActionView, ScopeSlugView, StageSlugView};
use serde_json::Value;

/// 観測アクション — 状態を動かさず directive だけを出す 3 つ。本ファイルの照合対象である。
const OBSERVATIONS: [&str; 3] = ["next", "next_parked", "done_stutter"];

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

fn plan_of(v: &Value) -> PlanActionView {
    match tag(v) {
        "Execute" => PlanActionView::Execute,
        "SkipPlan" => PlanActionView::Skip,
        t => panic!("unknown plan tag {t}"),
    }
}

/// モデル状態のうち、リードモデルに現れる分だけを写した中間表現。
///
/// 復号ヘルパはコマンド側 ITF と同型だが**複製で持つ** — 側ごと専用化が DRY に優先する
/// (`coding-rules/cqrs-boundaries.md`「共有部品は側の独立を DRY に優先」)。
struct ModelState {
    last_action: String,
    directive_tag: String,
    directive_stage: Option<usize>,
    cursor: usize,
    status: String,
    parked_at: i64,
    /// **実効**プラン。recompose のオーバレイは投影が既に Stage Progress 行末へ書き戻して
    /// いるので、リードモデル上の行末トークンに対応するのは `plan` ではなく `overlay` である。
    overlay: Vec<PlanActionView>,
    checkbox: Vec<CheckboxState>,
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
        overlay: map_to_vec(&v["overlay"], n, plan_of),
        checkbox: map_to_vec(&v["checkbox"], n, checkbox_of),
    }
}

fn slug(index: usize) -> String {
    format!("stage-{index}")
}

/// モデル状態を実行状態リードモデルのビューへ写す。
///
/// **park は `- **Status**:` ではなく `- **Parked At Stage**:` マーカーで表される**
/// ([`ExecutionStatus`] は Running / Completed の 2 値で、park とは直交 — BR1.0)。
/// したがってモデルの `WorkflowParked` は「Status = Running ∧ park マーカーがカーソル位置」
/// へ写り、parked 分岐は [`ExecutionStateView::parked_active`] の導出述語が発火させる。
fn view_of(m: &ModelState) -> ExecutionStateView {
    let stages: Vec<StageProgressView> = (0..m.checkbox.len())
        .map(|i| {
            // 索引 0 = initialization (非ゲート)、以降 = inception。モデルの
            // `gated(s) = s != 0` はこの合成計画への抽象である。
            let phase = if i == 0 {
                PhaseView::Initialization
            } else {
                PhaseView::Inception
            };
            StageProgressView::new(
                StageSlugView::parse(&slug(i)).unwrap(),
                phase,
                m.checkbox[i],
                m.overlay[i],
            )
        })
        .collect();
    let status = match m.status.as_str() {
        "Running" | "WorkflowParked" => ExecutionStatus::Running,
        "WorkflowCompleted" => ExecutionStatus::Completed,
        s => panic!("unknown status {s}"),
    };
    // `parkedAt = -1` は未 park。
    let parked = usize::try_from(m.parked_at).ok().map(slug);
    ExecutionStateView::new(
        ScopeSlugView::parse("classic").unwrap(),
        status,
        &slug(m.cursor),
        parked.as_deref(),
        "itf",
        stages,
    )
    .unwrap()
}

/// 組み立てたビューの観測面がモデル状態と一致することを **全ステップで**確かめる。
///
/// これが無いと、判断の照合は「自分が組み立てたビュー」に対する自己言及になり、写し間違い
/// (status の写像・実効プランに `plan` を使ってしまう等) を検出できない。コマンド側 ITF の
/// `assert_projection` と対をなす検査である。
fn assert_view_faces(view: &ExecutionStateView, m: &ModelState, step: usize) {
    let n = view.stage_count();
    assert_eq!(n, m.checkbox.len(), "step {step}: stage count");
    for s in 0..n {
        let stage = view.stage_index(s).unwrap();
        assert_eq!(
            view.checkbox(stage),
            Some(m.checkbox[s]),
            "step {step}: checkbox[{s}]"
        );
        assert_eq!(
            view.effective_plan(stage),
            Some(m.overlay[s]),
            "step {step}: overlay[{s}]"
        );
        // モデルの `gated(s) = s != 0` — ビューでは行の phase から導かれる (BR1.3)。
        assert_eq!(view.is_gated(stage), s != 0, "step {step}: gated[{s}]");
    }
    assert_eq!(view.cursor().to_usize(), m.cursor, "step {step}: cursor");
    let parked = view
        .parked_at()
        .map_or(-1, |p| i64::try_from(p.to_usize()).unwrap());
    assert_eq!(parked, m.parked_at, "step {step}: parkedAt");
    match m.status.as_str() {
        // park は Status と直交する — 未 park なら parked 分岐は発火しない。
        "Running" => {
            assert_eq!(
                view.status(),
                ExecutionStatus::Running,
                "step {step}: status"
            );
            assert!(!view.parked_active(), "step {step}: not parked");
        }
        // park 中でも Status 行は Running のまま。発火させるのは park マーカーである。
        "WorkflowParked" => {
            assert_eq!(
                view.status(),
                ExecutionStatus::Running,
                "step {step}: status"
            );
            assert!(view.parked_active(), "step {step}: parked active");
        }
        "WorkflowCompleted" => {
            assert_eq!(
                view.status(),
                ExecutionStatus::Completed,
                "step {step}: completed"
            );
        }
        s => panic!("unknown status {s}"),
    }
}

/// ビューの判断をモデルの `lastDirective` と突き合わせる。
fn assert_signal(view: &ExecutionStateView, m: &ModelState, step: usize) {
    let decision = view.next_decision(&NextRequest::default());
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

fn replay(path: &std::path::Path, seen: &mut std::collections::BTreeSet<String>) {
    let text = std::fs::read_to_string(path).unwrap();
    let trace: Value = serde_json::from_str(&text).unwrap();
    let states: Vec<ModelState> = trace["states"]
        .as_array()
        .unwrap()
        .iter()
        .map(parse_state)
        .collect();
    assert_eq!(states[0].last_action, "init");
    for (i, m) in states.iter().enumerate() {
        // ビューの写しはどのステップでも忠実であること。判断の照合はその上に乗る。
        let view = view_of(m);
        assert_view_faces(&view, m, i);
        if OBSERVATIONS.contains(&m.last_action.as_str()) {
            seen.insert(m.last_action.clone());
            assert_signal(&view, m, i);
        }
    }
}

#[test]
fn the_ladder_conforms_to_every_committed_engine_loop_trace() {
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
    // 観測アクションの網羅: 3 つとも少なくとも 1 つのコミット済みトレースに現れること。
    // 遷移アクション 13 種の網羅はコマンド側 ITF が同じ形で固定している (両側で維持する)。
    for action in OBSERVATIONS {
        assert!(
            seen.contains(action),
            "no committed trace exercises action {action}"
        );
    }
}
