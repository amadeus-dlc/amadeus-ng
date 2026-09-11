//! CLIの固定本家2.7.1比較。原本の期待バイトは変更しない。
//!
//! `tests/golden/upstream-a277af21/cli/` を読み、採取元SHAも検査する。
//! 不正continueとparkは同じ入力条件で公開JSON全文を比較する。
//! load-steering/run-stageは合成グラフを使うため、キー集合の検査に限定される。
//! personaの本文は本家採取と同じ配布内容を置き、全文を比較する。
//! narration/conductor_personaの欠落を許す例外は設けない。
//!
//! reportの既存比較はpractices-discoveryをdomain-designへ読み替える限定検査である。
//! 同一の開始状態・全公開ファイル比較の代わりにはしない。承認待ちの再報告は
//! 本家2.7.1でgate evidenceの再検証へ変わっており、現在の単純no-opとは一致しない。
//! 非ゲート報告、フェーズをまたぐ報告、次工程の全配布入力の再現は別途必要である。
#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use core_infrastructure::canon_json::{JsonValue, parse};

/// 採取済みケースの置き場 (リポジトリ根からの相対)。
fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join("tests/golden/upstream-a277af21/cli")
}

#[test]
fn the_cli_corpus_uses_the_approved_upstream_revision() {
    let metadata = fs::read_to_string(golden_dir().join("cases-missing.json")).expect("採取元情報");
    let source: serde_json::Value = serde_json::from_str(&metadata).expect("JSON");
    assert_eq!(
        source
            .get("upstream_commit")
            .and_then(serde_json::Value::as_str)
            .expect("採取元SHA"),
        "a277af218f0df7f325d3b8be7b6d90fce2c5bd40"
    );
}

/// 採取済みケースの stdout (1 行 JSON) をそのまま読む。
fn recorded(case: &str) -> String {
    let path = golden_dir().join(case).join("stdout.json");
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("ゴールデン {} が読めない: {error}", path.display()))
        .trim_end_matches('\n')
        .to_string()
}

/// 名指しキーの文字列値。
fn string_of(line: &str, key: &str) -> String {
    match parse(line).expect("ゴールデンは JSON") {
        JsonValue::Object(members) => match members.get(key) {
            Some(JsonValue::String(text)) => text.clone(),
            other => panic!("{key} は文字列であるべき: {other:?}"),
        },
        other => panic!("オブジェクトであるべき: {other:?}"),
    }
}

/// オブジェクトのキー集合 (順序は upstream の挿入順であって契約ではない — §キー順)。
fn keys(line: &str) -> BTreeSet<String> {
    match parse(line).expect("ゴールデンは JSON") {
        JsonValue::Object(members) => members.iter().map(|(key, _)| key.to_string()).collect(),
        other => panic!("オブジェクトであるべき: {other:?}"),
    }
}

/// 定義 3 入力と memory 層を書いた fresh なワークスペース。
///
/// キー集合の比較は計画の大きさに依らないので、採取時の 33 ノードのグラフではなく最小の
/// 合成グラフを使う。この検査だけでは配布グラフ全体の実行を証明しない。
struct Workspace {
    root: tempfile::TempDir,
}

impl Workspace {
    fn create() -> Workspace {
        let workspace = Workspace {
            root: tempfile::tempdir().expect("一時ディレクトリ"),
        };
        workspace.write_definition();
        let common = workspace.path(".claude/aidlc-common");
        fs::create_dir_all(&common).expect("配布共通ファイル");
        fs::write(
            common.join("conductor.md"),
            string_of(&recorded("continue/load-steering"), "conductor_persona"),
        )
        .expect("固定本家のpersona本文");
        let memory = workspace.path("aidlc/spaces/default/memory");
        fs::create_dir_all(&memory).expect("memory");
        fs::write(
            memory.join("org.md"),
            "# Org\n\n## Way of Working\n\n規則。\n",
        )
        .expect("org.md");
        fs::create_dir_all(workspace.path("aidlc/spaces/default/intents")).expect("intents");
        workspace
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.path().join(relative)
    }

    fn project_dir(&self) -> &Path {
        self.root.path()
    }

    /// 鋳造した intent の記録ディレクトリ (`active-intent` カーソルの指す先)。
    fn record_dir(&self) -> PathBuf {
        let intents = self.path("aidlc/spaces/default/intents");
        let name = fs::read_to_string(intents.join("active-intent")).expect("active-intent");
        intents.join(name.trim())
    }

    fn write_definition(&self) {
        let data = self.path(".claude/tools/data");
        let scopes = self.path(".claude/scopes");
        fs::create_dir_all(&data).expect("data");
        fs::create_dir_all(&scopes).expect("scopes");
        fs::write(
            data.join("harness.json"),
            r#"{"name":"claude","harnessDir":".claude","rulesSubdir":"rules"}"#,
        )
        .expect("harness.json");
        // 採取済みの `run-stage` が載せるキーを全部出させるため、reviewer と support_agents を
        // 持つノードにする (任意キーは値が在るときだけ載る)。
        let node = |slug: &str, number: &str, name: &str, phase: &str, extra: &str| {
            format!(
                r#"{{"slug":"{slug}","number":"{number}","name":"{name}","phase":"{phase}",
                     "execution":"ALWAYS","mode":"subagent","lead_agent":"orchestrator",
                     "scopes":["classic"]{extra}}}"#
            )
        };
        let reviewed = r#","support_agents":["aidlc-quality-agent"],
             "reviewer":"aidlc-architecture-reviewer-agent","review_class":"advisory",
             "reviewer_max_iterations":2,"produces":["design"],"review_artifact":"design""#;
        fs::write(
            data.join("stage-graph.json"),
            format!(
                "[{},{},{}]",
                node("state-init", "0.1", "State Init", "initialization", ""),
                node(
                    "domain-design",
                    "1.1",
                    "Domain Design",
                    "inception",
                    reviewed
                ),
                node("contract-design", "1.2", "Contract Design", "inception", ""),
            ),
        )
        .expect("stage-graph.json");
        fs::write(
            data.join("scope-grid.json"),
            r#"{"classic":{"stages":{"state-init":"EXECUTE","domain-design":"EXECUTE","contract-design":"EXECUTE"}}}"#,
        )
        .expect("scope-grid.json");
        fs::write(
            scopes.join("aidlc-classic.md"),
            "---\nname: classic\n---\n\n# Classic\n",
        )
        .expect("scope identity");
    }

    async fn invoke(&self, argv0: &str, args: &[&str]) -> aidlc::runtime::Completion {
        let mut owned: Vec<String> = args.iter().map(|a| (*a).to_string()).collect();
        owned.push("--project-dir".to_string());
        owned.push(self.project_dir().to_string_lossy().into_owned());
        aidlc::runtime::run(argv0, &owned, self.project_dir()).await
    }

    async fn mint(&self) {
        let completion = self
            .invoke(
                "aidlc-utility",
                &["intent-create", "--scope", "classic", "--label", "golden"],
            )
            .await;
        assert_eq!(completion.code(), 0, "鋳造は通る: {completion:?}");
    }
}

/// 出た 1 行を取り出す。
fn line(completion: &aidlc::runtime::Completion) -> String {
    completion
        .line()
        .unwrap_or_else(|| panic!("stdout に 1 行が要る: {completion:?}"))
        .to_string()
}

/// **バイト一致** — 逐語文言だけの directive はフィクスチャに依らない。
#[tokio::test]
async fn the_invalid_continuation_token_is_byte_identical_to_the_recorded_case() {
    let workspace = Workspace::create();
    workspace.mint().await;

    let completion = workspace
        .invoke(
            "aidlc-orchestrate",
            &["continue", "not-a-continuation-token"],
        )
        .await;

    assert_eq!(completion.code(), 0, "ビジネス拒否は exit 0");
    assert_eq!(
        line(&completion),
        recorded("continue/invalid-token"),
        "採取済みの stdout と 1 バイトも違わない"
    );
}

/// `load-steering` のキー集合が採取済みケースと一致する。
#[tokio::test]
async fn the_load_steering_keys_match_the_recorded_case() {
    let workspace = Workspace::create();
    workspace.mint().await;

    let completion = workspace.invoke("aidlc-orchestrate", &["next"]).await;

    let emitted = keys(&line(&completion));
    let expected = keys(&recorded("next/start"));
    assert_eq!(
        emitted, expected,
        "load-steering のキー集合は採取済みケースと同じである"
    );
}

/// 終端 `run-stage` のキー集合と、同じ配布本文から読むpersona。
///
/// 採取済みの `continue/load-steering` は「最後の部まで配り終えた継続」なので、返るのは
/// 続きの部ではなく台帳付きの `run-stage` である。こちらも同じ形で終端に着く。
#[tokio::test]
async fn the_terminal_run_stage_keys_and_persona_match_the_recorded_case() {
    let workspace = Workspace::create();
    workspace.mint().await;

    // 1 部で収まる規則束なので、最初の `next` が第 1 部を配って終端に達する。
    let first = workspace.invoke("aidlc-orchestrate", &["next"]).await;
    let token = match parse(&line(&first)).expect("JSON") {
        JsonValue::Object(members) => match members.get("continue_token") {
            Some(JsonValue::String(token)) => token.clone(),
            other => panic!("continue_token は文字列であるべき: {other:?}"),
        },
        other => panic!("オブジェクトであるべき: {other:?}"),
    };
    let completion = workspace
        .invoke("aidlc-orchestrate", &["continue", &token])
        .await;

    let emitted = keys(&line(&completion));
    let expected = keys(&recorded("continue/load-steering"));
    let missing: Vec<&String> = expected.difference(&emitted).collect();
    assert!(
        missing.is_empty(),
        "本家の必須キーが欠けている: {missing:?}"
    );
    assert_eq!(
        string_of(&line(&completion), "conductor_persona"),
        string_of(&recorded("continue/load-steering"), "conductor_persona")
    );
    // 採取済みのステージ (`practices-discovery`) はレビュアを宣言しないので、任意の 3 キーが
    // 現れない。こちらのフィクスチャは宣言するので現れる — その 3 つ以外は増やさない。
    let extra: Vec<&String> = emitted.difference(&expected).collect();
    assert_eq!(
        extra,
        vec![
            &"review_class".to_string(),
            &"reviewer".to_string(),
            &"reviewer_max_iterations".to_string(),
        ],
        "採取済みケースに無いキーは、レビュア宣言に応じた任意の 3 つだけである"
    );
}

/// `parked` directive — 説明文を含む公開JSON全文。
///
/// 採取済みの `cli/park/park` は 33 ノードのグラフの `domain-design` で止まっている。こちらの
/// 合成グラフでも誕生のカーソルは `domain-design`（最初のゲート付き in-scope ステージ）なので、
/// すべての項目を1バイトも違わないところまで突き合わせられる。
#[tokio::test]
async fn the_parked_directive_is_byte_identical_to_the_recorded_case() {
    let workspace = Workspace::create();
    workspace.mint().await;
    let completion = workspace.invoke("aidlc-orchestrate", &["park"]).await;
    assert_eq!(completion.code(), 0, "{completion:?}");
    assert_eq!(
        line(&completion),
        recorded("park/park"),
        "同じstageでparkした公開JSON全文"
    );
}

/// 採取時のステージ slug（33 ノードのグラフの最初のゲート付きステージ）。
const RECORDED_SLUG: &str = "practices-discovery";
/// 合成グラフの同じ位置のステージ。
const SYNTHETIC_SLUG: &str = "domain-design";

/// 採取済みケースの本文を、slug だけ合成グラフのものへ読み替えて返す。
///
/// 置換するのは**ステージ名だけ**である — 逐語の文型・句読点・`(scope: classic)` の綴りは
/// 1 バイトも触らない。scope はどちらも `classic` なので置換が要らない。
fn recorded_for_synthetic_graph(case: &str) -> String {
    recorded(case).replace(RECORDED_SLUG, SYNTHETIC_SLUG)
}

/// advisory 1 パスの受領証（依頼 → READY 判定）を積む（b48 の段 11 を通すため）。
///
/// 依頼の受領証は宣言された `produces` の実バイトに束縛されるので、先に成果物を置く。
async fn record_advisory_receipt(workspace: &Workspace, stage: &str) {
    let artifact_dir = workspace.record_dir().join("inception").join(stage);
    fs::create_dir_all(&artifact_dir).expect("成果物ディレクトリ");
    fs::write(artifact_dir.join("design.md"), "# Design\n\n設計。\n").expect("成果物");
    for extra in [Vec::new(), vec!["--verdict", "READY"]] {
        let mut argv = vec![
            "review",
            "--stage",
            stage,
            "--reviewer",
            "aidlc-architecture-reviewer-agent",
            "--iteration",
            "1",
        ];
        argv.extend_from_slice(&extra);
        let completion = workspace.invoke("aidlc-log", &argv).await;
        assert_eq!(completion.code(), 0, "受領証は積める: {completion:?}");
        // 判定は、依頼時のバイトに `## Review` 付録だけが追記された成果物へ束縛される。
        if extra.is_empty() {
            let mut body = fs::read(artifact_dir.join("design.md")).expect("成果物");
            body.extend_from_slice(
                b"\n## Review\n\n**Verdict:** READY\n**Reviewer:** aidlc-architecture-reviewer-agent\n**Iteration:** 1\n",
            );
            fs::write(artifact_dir.join("design.md"), body).expect("付録");
        }
    }
}

/// `report` を 1 回叩いて出た 1 行を返す。
async fn report_line(workspace: &Workspace, args: &[&str]) -> String {
    let mut argv = vec!["report"];
    argv.extend_from_slice(args);
    let completion = workspace.invoke("aidlc-orchestrate", &argv).await;
    assert_eq!(completion.code(), 0, "{completion:?}");
    line(&completion)
}

/// ケースを分け、先の失敗が後の比較を隠さないようにする。
#[tokio::test]
async fn opening_a_gate_matches_the_recorded_reply_after_slug_substitution() {
    let workspace = Workspace::create();
    workspace.mint().await;
    assert_eq!(
        report_line(&workspace, &["--result", "awaiting-approval"]).await,
        recorded_for_synthetic_graph("report/awaiting-approval")
    );
}

#[tokio::test]
async fn reopening_an_awaiting_gate_matches_the_revalidated_reply_after_slug_substitution() {
    let workspace = Workspace::create();
    workspace.mint().await;
    report_line(&workspace, &["--result", "awaiting-approval"]).await;
    assert_eq!(
        report_line(&workspace, &["--result", "awaiting-approval"]).await,
        recorded_for_synthetic_graph("report/awaiting-approval-repeat")
    );
}

#[tokio::test]
async fn approving_a_gate_matches_the_recorded_reply_after_slug_substitution() {
    let workspace = Workspace::create();
    workspace.mint().await;
    report_line(&workspace, &["--result", "awaiting-approval"]).await;
    record_advisory_receipt(&workspace, SYNTHETIC_SLUG).await;
    assert_eq!(
        report_line(&workspace, &["--result", "approved", "--user-input", "A"]).await,
        recorded_for_synthetic_graph("report/approved")
    );
}

#[tokio::test]
async fn rejecting_a_gate_matches_the_recorded_reply_after_slug_substitution() {
    let workspace = Workspace::create();
    workspace.mint().await;
    assert_eq!(
        report_line(
            &workspace,
            &[
                "--result",
                "rejected",
                "--reason",
                "Sharpen the testing posture."
            ]
        )
        .await,
        recorded_for_synthetic_graph("report/rejected")
    );
}

#[tokio::test]
async fn revising_a_gate_matches_the_recorded_reply_after_slug_substitution() {
    let workspace = Workspace::create();
    workspace.mint().await;
    report_line(
        &workspace,
        &[
            "--result",
            "rejected",
            "--reason",
            "Sharpen the testing posture.",
        ],
    )
    .await;
    assert_eq!(
        report_line(
            &workspace,
            &[
                "--result",
                "revised",
                "--user-input",
                "Tightened the testing posture."
            ]
        )
        .await,
        recorded_for_synthetic_graph("report/revised")
    );
}
