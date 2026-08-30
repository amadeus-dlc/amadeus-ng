//! 投影の**ゴールデン逐語一致** — 1 ドメインイベントから監査行と状態ファイル差分の両面を描き、
//! upstream の実バイト（`audit.md` と `state.diff`）と突き合わせる（FR1.1 / NFR3）。
//!
//! # 計画は出荷グラフのゴールデンから組む
//!
//! 表示属性（ステージ番号・表題・担当エージェント）は `Started` が運ぶ（オーナー裁定
//! 2026-08-29）。テストではその値を手で書かず、**upstream の出荷グラフそのもの**
//! （`tests/golden/upstream-3c3146cf/stage-graph.json` の 33 ノード）と
//! スコープグリッド（`scope-grid.json` の classic 列）から組む。手写しの値で合わせにいくと
//! 「テストに合わせた実装」になってしまうためである。
//!
//! `reverse-engineering` はグリッド上 EXECUTE だが CONDITIONAL であり、ゴールデンの
//! フィクスチャは greenfield なので SKIP へ畳む（状態ファイルの
//! `- [ ] reverse-engineering — SKIP` と `2.1 (reverse-engineering — greenfield)` がその実測）。
//! これで in-scope は 25 になり、`- **Total Stages**: 25` と一致する。
//!
//! # `state.diff` の扱い
//!
//! unified diff なのでハンクから「前」と「後」の断片を組み立て直し、前の断片へ投影を当てた
//! 結果が後の断片と 1 バイトも違わないことを見る。状態ファイル writer は行単位で働くので、
//! ハンクの断片だけでも観測は成立する。

// テストコードでは unwrap / expect / panic を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};

use chrono::{DateTime, SecondsFormat, Utc};
use core_command_domain::orchestration::{
    Created, GateApproved, GateOpened, GateRejected, Intent, IntentExecutionEvent,
    IntentExecutionId, IntentId, Jumped, Parked, Recomposed, StageCompleted, StageDisplay,
    StageEntry, StageRevised, StageSkipped, StartRequest, Started, WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
    WorkflowDefinitionId,
};
use core_read_model_updater::orchestration::{GlobalSeqNr, JournalEntry};
use core_read_model_updater::workspace::{ReadModel, ResolvedPlan, project};

/// ゴールデンが正規化で潰した実行時値の置き換え先。
const TS_PLACEHOLDER: &str = "<TS>";

/// 投影に渡す発生時刻（正規化で `<TS>` に潰れるので値そのものは観測されない）。
const AT: &str = "2026-08-22T13:43:00Z";

/// 行を運ぶ集約識別子（投影は識別子を描かないので値は任意）。
const INTENT: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

/// ゴールデンのフィクスチャが使う実行識別子 (ジャーナル行の集約キー)。
const EXECUTION: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000";

/// ゴールデンのフィクスチャが使うスコープ。
const SCOPE: &str = "classic";

/// 同フィクスチャの人間要求（`cli/intent-create` の実バイト）。
const REQUEST: &str = "/aidlc Build a small ordering service";

/// グリッド上 EXECUTE だが CONDITIONAL であり、greenfield では畳まれるステージ。
const CONDITIONAL_ON_BROWNFIELD: &str = "reverse-engineering";

fn golden_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../tests/golden/upstream-3c3146cf")
}

fn golden(case: &str) -> PathBuf {
    golden_root().join("cli").join(case)
}

fn at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(AT)
        .expect("固定の ISO 8601")
        .with_timezone(&Utc)
}

fn slug(value: &str) -> StageSlug {
    StageSlug::parse(value).expect("テストの slug は文法内")
}

fn entry(event: IntentExecutionEvent) -> JournalEntry {
    JournalEntry::new(
        GlobalSeqNr::new(1),
        IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
        1,
        at(),
        event,
    )
}

/// 出荷グラフとスコープグリッドから intent（誕生の材料）を組む（表示属性は upstream の
/// 実データ）。計画・走査結果の正本は intent 側で、`Started` は識別子だけを運ぶ (issue #56)。
fn genesis_intent() -> Intent {
    let nodes: Vec<serde_json::Value> = serde_json::from_str(
        &std::fs::read_to_string(golden_root().join("stage-graph.json")).expect("stage-graph"),
    )
    .expect("stage-graph は JSON");
    let grid: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(golden_root().join("scope-grid.json")).expect("scope-grid"),
    )
    .expect("scope-grid は JSON");
    let plan_of = &grid[SCOPE]["stages"];

    let stages: Vec<StageEntry> = nodes
        .iter()
        .map(|node| {
            let name = node["slug"].as_str().expect("slug");
            let executes =
                plan_of[name].as_str() == Some("EXECUTE") && name != CONDITIONAL_ON_BROWNFIELD;
            StageEntry::new(
                slug(name),
                PhaseId::parse(node["phase"].as_str().expect("phase")).expect("フェーズ名"),
                if executes {
                    PlanAction::Execute
                } else {
                    PlanAction::Skip
                },
                name == CONDITIONAL_ON_BROWNFIELD,
                StageDisplay::new(
                    StageNumber::parse(node["number"].as_str().expect("number")).expect("番号"),
                    node["name"].as_str().expect("name"),
                    node["lead_agent"].as_str().expect("lead_agent"),
                )
                .expect("出荷グラフの表示属性は単一行"),
            )
        })
        .collect();

    Intent::from(Created::new(
        IntentId::parse(INTENT).expect("UUIDv7"),
        WorkflowDefinitionId::parse("claude").expect("定義 id"),
        DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision"),
        StartRequest::new(SCOPE, REQUEST),
        stages,
        WorkspaceScan::new(
            BrownfieldGreenfield::Greenfield,
            "Unknown",
            "Unknown",
            "Unknown",
        )
        .expect("単一行"),
    ))
}

fn started() -> Started {
    Started::new(IntentId::parse(INTENT).expect("UUIDv7"))
}

fn plan() -> ResolvedPlan {
    ResolvedPlan::of(&genesis_intent())
}

/// 実測値へゴールデンと同じ正規化（ISO 8601 UTC → `<TS>`）を当てる。
fn normalise(rendered: &str) -> String {
    rendered.replace(
        &at().to_rfc3339_opts(SecondsFormat::Secs, true),
        TS_PLACEHOLDER,
    )
}

/// unified diff のハンクから「前」と「後」の断片を組み立てる。
fn before_and_after(diff: &str) -> (String, String) {
    let mut before = String::new();
    let mut after = String::new();
    let mut in_hunk = false;
    for line in diff.lines() {
        if line.starts_with("@@") {
            in_hunk = true;
            continue;
        }
        if !in_hunk || line.starts_with("---") || line.starts_with("+++") {
            continue;
        }
        match line.split_at_checked(1) {
            Some(("-", rest)) => {
                before.push_str(rest);
                before.push('\n');
            }
            Some(("+", rest)) => {
                after.push_str(rest);
                after.push('\n');
            }
            Some((" ", rest)) => {
                before.push_str(rest);
                before.push('\n');
                after.push_str(rest);
                after.push('\n');
            }
            _ => {}
        }
    }
    (before, after)
}

/// 1 ケースを検収する — 監査行と状態ファイルの**両面**をバイトで突き合わせる。
fn assert_case(case: &str, event: IntentExecutionEvent) {
    assert_case_with_context(case, event, "");
}

/// ハンクの外にある行が状態面の結果を左右するケース用。
///
/// `- **Completed**:` は upstream と同じく**チェックボックスの数え直し**で同期する
/// (`countCheckboxes`)。ところが `state.diff` のハンクは変わった行の周辺しか含まないので、
/// 既に `[x]` になっている別セクションの行が断片に入らない。`context` はその**実ファイルには
/// あるがハンクに写っていない行**を補うためのもので、値を捏造するのではなく数え合わせの前提を
/// 揃える。補った行が正しいかどうかは、結果が upstream の `- **Completed**:` と一致するか
/// どうかで検証される。
fn assert_case_with_context(case: &str, event: IntentExecutionEvent, context: &str) {
    let dir = golden(case);
    let expected_audit = std::fs::read_to_string(dir.join("audit.md")).expect("audit.md");
    let diff = std::fs::read_to_string(dir.join("state.diff")).expect("state.diff");
    let (before, after) = before_and_after(&diff);
    let before = format!("{context}{before}");
    let expected_state = format!("{context}{after}");

    let mut model = ReadModel::new(before);
    project(&[entry(event)], &plan(), &mut model).unwrap_or_else(|error| {
        panic!("{case}: 投影が失敗した: {error}");
    });

    assert_eq!(
        normalise(model.appended_audit()),
        expected_audit,
        "{case}: 監査行が upstream と違う"
    );
    assert_eq!(
        normalise(model.state()),
        expected_state,
        "{case}: 状態ファイルが upstream と違う"
    );
}

/// 監査行だけを検収する（状態面が本 Bolt の射程外のケース）。
///
/// 空のシャードへの初回書込だけはゴールデンがヘッダ行 `# AI-DLC Audit Log\n` を先頭に持つ。
/// ヘッダを置くのはシャードライタの仕事であって投影ではないので、比較の前に剥がす。
fn assert_audit_only(case: &str, event: IntentExecutionEvent, state: &str) {
    let raw = std::fs::read_to_string(golden(case).join("audit.md")).expect("audit.md");
    let expected_audit = raw
        .strip_prefix(core_read_model_updater::workspace::SHARD_HEADER)
        .unwrap_or(&raw)
        .to_string();
    let mut model = ReadModel::new(state.to_string());
    let outcome = project(&[entry(event)], &plan(), &mut model);
    assert!(
        outcome.is_ok()
            || matches!(outcome, Err(ref error) if error.to_string() == "scaffold missing"),
        "{case}: 想定外の失敗: {outcome:?}"
    );
    assert_eq!(
        normalise(model.appended_audit()),
        expected_audit,
        "{case}: 監査行が upstream と違う"
    );
}

// ---------------------------------------------------------------------------
// 両面一致
// ---------------------------------------------------------------------------

#[test]
fn opening_a_gate_writes_the_awaiting_approval_row_and_moves_the_checkbox() {
    assert_case(
        "report/awaiting-approval",
        IntentExecutionEvent::GateOpened(GateOpened::new(slug("practices-discovery"), vec![])),
    );
}

#[test]
fn rejecting_a_gate_writes_two_rows_and_bumps_the_revision_count() {
    assert_case(
        "report/rejected",
        IntentExecutionEvent::GateRejected(GateRejected::new(
            slug("practices-discovery"),
            Some("Sharpen the testing posture.".to_string()),
        )),
    );
}

#[test]
fn revising_a_stage_re_enters_the_gate_with_the_verbatim_details() {
    assert_case(
        "report/revised",
        IntentExecutionEvent::StageRevised(StageRevised::new(slug("practices-discovery"))),
    );
}

#[test]
fn approving_a_gate_completes_the_stage_and_starts_the_next_one() {
    // `- **Completed**: 3 → 4` はチェックボックスの数え直しである。既に完了している
    // initialization 3 ステージの行は `## INITIALIZATION PHASE` セクションにあり、
    // ハンクに写っていないので補う（補い方が正しければ 4 になる — それが検証になる）。
    assert_case_with_context(
        "report/approved",
        IntentExecutionEvent::GateApproved(GateApproved::new(
            slug("practices-discovery"),
            Some("A".to_string()),
        )),
        concat!(
            "### INITIALIZATION PHASE\n",
            "- [x] workspace-scaffold — EXECUTE\n",
            "- [x] workspace-detection — EXECUTE\n",
            "- [x] state-init — EXECUTE\n",
        ),
    );
}

#[test]
fn completing_an_ungated_stage_writes_its_own_details_wording() {
    // 非ゲートの完了は `Stage <表示名> completed`、ゲート経由は `Stage <表示名> approved by
    // gate` で文言が割れる（`cli/report/completed-ungated` と `cli/report/approved` の実バイト）。
    // `- **Active Agent**:` はハンクの外（値が変わらないため差分に写らない）なので補う。
    // 補った値 `orchestrator` は同じ採取列の `cli/jump/execute-backward` の後断片が示す実値で
    // あり、投影が次ステージ workspace-detection の担当を書き直した結果と一致するかで検証される。
    assert_case_with_context(
        "report/completed-ungated",
        IntentExecutionEvent::StageCompleted(StageCompleted::new(slug("workspace-scaffold"))),
        "- **Active Agent**: orchestrator\n",
    );
}

#[test]
fn skipping_a_stage_moves_on_without_touching_the_completed_count() {
    assert_case(
        "skip/skipped",
        IntentExecutionEvent::StageSkipped(StageSkipped::new(
            slug("user-stories"),
            "No UI surface in this workflow.".to_string(),
        )),
    );
}

#[test]
fn jumping_forward_skips_the_source_and_opens_the_target() {
    // `- **Completed**: 4` は数え直しで、ハンクに写っているのは practices-discovery の 1 本
    // だけなので initialization 3 本を補う（補い方が正しければ 4 になる — それが検証になる）。
    assert_case_with_context(
        "jump/execute-forward",
        IntentExecutionEvent::Jumped(Jumped::new(slug("domain-design"))),
        concat!(
            "### INITIALIZATION PHASE\n",
            "- [x] workspace-scaffold — EXECUTE\n",
            "- [x] workspace-detection — EXECUTE\n",
            "- [x] state-init — EXECUTE\n",
        ),
    );
}

#[test]
fn jumping_backward_resets_the_downstream_and_hands_the_phase_row_back() {
    // フェーズ境界をまたぐジャンプの 3 行はゲート経由のものと同型ではない — ジャンプ側だけが
    // `**Details**:` を持ち、`**Stages completed**:` は数え直しである (ここでは全部 [ ] に
    // 戻したので 0)。`- **Last Completed Stage**:` は到達点が先頭ステージで手前に完了が無い
    // ため upstream の既定値 `state-init` になる。
    assert_case(
        "jump/execute-backward",
        IntentExecutionEvent::Jumped(Jumped::new(slug("workspace-scaffold"))),
    );
}

#[test]
fn jumping_forward_across_a_phase_verifies_the_one_it_leaves() {
    // 前方で境界をまたぐと出発フェーズは `Verified`、飛び越えた ideation は `Skipped` の
    // ままである (スコープ内ステージが 0 本なので触らない)。`**Stages completed**:` は 1 で、
    // 直前の [S] 化のあとに残った [x] (workspace-scaffold) 1 本を数えた値でしか説明が付かない。
    // `stages_skipped` の並びは upstream の emit 順 — 間のステージを順に並べたあと、
    // **最後に出発点そのもの**が来る。
    assert_case(
        "jump/execute-forward-across-phases",
        IntentExecutionEvent::Jumped(Jumped::new(slug("contract-design"))),
    );
}

#[test]
fn approving_the_last_stage_of_a_phase_counts_the_checkboxes_not_the_plan() {
    // ゲート経由の境界 3 本はジャンプ側と違い `**Details**:` を持たない。`**Stages completed**:`
    // は 2 で、計画上の inception 内スコープ件数 8 とは一致しない — 倒したあとのチェックボックス
    // を数えた値だけがこれを説明する。数え直しの材料 (既に [x] の workspace-scaffold) は
    // ハンクの外なので補う。
    assert_case_with_context(
        "report/approved-across-phases",
        IntentExecutionEvent::GateApproved(GateApproved::new(
            slug("delivery-planning"),
            Some("A".to_string()),
        )),
        "- [x] workspace-scaffold — EXECUTE\n",
    );
}

#[test]
fn recomposing_keeps_existing_skip_entries_where_they_are() {
    // 既に skip 済みの 4.5 より前の番号 4.3 を後から skip しても、Skip 行は番号順に並べ替え
    // られない — 既存項目はその位置のまま残り、新規が graph 順で末尾に付く。
    assert_case(
        "recompose/skip-two-appends-in-graph-order",
        IntentExecutionEvent::Recomposed(Recomposed::new(
            vec![slug("deployment-execution"), slug("feedback-optimization")],
            Vec::new(),
        )),
    );
}

#[test]
fn recomposing_back_into_scope_drops_the_annotated_entry_whole() {
    // `2.1 (reverse-engineering — greenfield)` は注釈ごと消え、2.1 は Execute 行の**末尾では
    // なく** 0.3 と 2.2 の間へ入る (Execute 行は毎回 graph 順に組み直されるため)。注釈から
    // slug を取り出せないと項目が落ちずに両方の行へ載る。
    // 実効計画は行末トークンが正本だが、先行する recompose で SKIP へ倒した 3 本の行は
    // ハンクの外にある。補う 3 行は同じ採取列の `recompose/skip-one` と
    // `skip-two-appends-in-graph-order` が示した実バイトそのもので、補い方が正しければ
    // `- **Total Stages**: 23` に落ちる — それが検証になる。
    assert_case_with_context(
        "recompose/add-restores-conditional",
        IntentExecutionEvent::Recomposed(Recomposed::new(
            Vec::new(),
            vec![slug("reverse-engineering")],
        )),
        concat!(
            "- [ ] deployment-execution — SKIP\n",
            "- [ ] incident-response — SKIP\n",
            "- [ ] feedback-optimization — SKIP\n",
        ),
    );
}

#[test]
fn recomposing_moves_a_stage_between_the_two_plan_lists() {
    assert_case(
        "recompose/skip-one",
        IntentExecutionEvent::Recomposed(Recomposed::new(
            vec![slug("incident-response")],
            Vec::new(),
        )),
    );
}

#[test]
fn parking_writes_the_marker_at_the_end_of_the_runtime_section() {
    assert_case(
        "park/park",
        IntentExecutionEvent::Parked(Parked::new(slug("domain-design"))),
    );
}

#[test]
fn unparking_removes_both_marker_lines() {
    assert_case("unpark/unpark", IntentExecutionEvent::Unparked);
}

// ---------------------------------------------------------------------------
// 監査行のみ（状態面が本 Bolt の射程外）
// ---------------------------------------------------------------------------

#[test]
fn the_genesis_draws_all_sixteen_initialization_rows() {
    // 状態面は空の本文へ当てるので `ScaffoldMissing` で止まる — 骨格を書くのは合成ルート
    // であって投影ではない（オーナー裁定 2026-08-29）。監査行 16 本は計画と走査結果だけから
    // 導けるので、ここで検収するのは監査面である。骨格の実バイトは同ケースの `state-full.md`
    // にあり、U7 が骨格を書くときの正本になる。
    assert_audit_only(
        "intent-create/classic-scope",
        IntentExecutionEvent::Started(started()),
        "",
    );
}

// ---------------------------------------------------------------------------
// 冪等（NFR3）
// ---------------------------------------------------------------------------

#[test]
fn projecting_the_same_entries_from_the_same_state_twice_yields_the_same_bytes() {
    let event = IntentExecutionEvent::GateApproved(GateApproved::new(
        slug("practices-discovery"),
        Some("A".to_string()),
    ));
    let diff =
        std::fs::read_to_string(golden("report/approved").join("state.diff")).expect("state.diff");
    let (before, _) = before_and_after(&diff);

    let run = |source: &str| {
        let mut model = ReadModel::new(source.to_string());
        project(&[entry(event.clone())], &plan(), &mut model).expect("投影");
        (
            model.state().to_string(),
            model.appended_audit().to_string(),
        )
    };
    assert_eq!(run(&before), run(&before));
}

#[test]
fn a_stage_outside_the_plan_is_refused_rather_than_drawn_wrong() {
    let mut model = ReadModel::new("## Stage Progress\n- [-] ghost — EXECUTE\n".to_string());
    let error = project(
        &[entry(IntentExecutionEvent::GateApproved(
            GateApproved::new(slug("ghost"), None),
        ))],
        &plan(),
        &mut model,
    )
    .expect_err("計画に無いステージ");
    assert_eq!(error.to_string(), "unknown stage: ghost");
}

/// ゴールデンのディレクトリが本当にそこにあることの保険（パスずれで空回りしない）。
#[test]
fn the_golden_corpus_is_where_the_test_thinks_it_is() {
    let root: &Path = &golden("report/approved");
    assert!(root.join("audit.md").exists(), "実際: {}", root.display());
    // 出荷グラフ 33 ノードのうち classic の in-scope は 25（`- **Total Stages**: 25`）。
    assert_eq!(plan().stages().len(), 33);
    assert_eq!(plan().in_scope_count(), 25);
}
