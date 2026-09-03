//! `next` のラダーを**実 CLI 経路**で 1 分岐ずつ固定する。
//!
//! b44 でラダーはクエリ側のユースケースから合成ルートのコントローラへ移った。分岐の入口は
//! すべて**要求の形**（フラグの有無・本文）で決まり、状態で決まる答えは行の綴り
//! (`read_next_answer.decision_kind`) から来る。したがって検収も「argv を与えて出た
//! directive を見る」形が正しい — 途中の型を突くと、分岐の入口とリードモデルの引当の
//! どちらが効いたのか分からなくなる。
//!
//! 旧 `next_use_case.rs` が持っていた 21 分岐の単体テストは、対象ごと消えた（b44(D)）。
//! ここがその置き換えである。
#![allow(clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

use core_infrastructure::canon_json::{JsonValue, parse};

struct Workspace {
    root: tempfile::TempDir,
}

impl Workspace {
    /// 定義 3 入力（scope は `classic` / `express` の 2 列）と memory 層を書いた fresh な根。
    fn create() -> Workspace {
        let workspace = Workspace {
            root: tempfile::tempdir().expect("一時ディレクトリ"),
        };
        workspace.write_definition();
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
        let node = |slug: &str, number: &str, name: &str, phase: &str| {
            format!(
                r#"{{"slug":"{slug}","number":"{number}","name":"{name}","phase":"{phase}",
                     "execution":"ALWAYS","mode":"inline","lead_agent":"orchestrator",
                     "scopes":["classic","express"]}}"#
            )
        };
        fs::write(
            data.join("stage-graph.json"),
            format!(
                "[{},{},{}]",
                node("state-init", "0.1", "State Init", "initialization"),
                node("domain-design", "1.1", "Domain Design", "inception"),
                node("contract-design", "1.2", "Contract Design", "inception"),
            ),
        )
        .expect("stage-graph.json");
        fs::write(
            data.join("scope-grid.json"),
            r#"{"classic":{"stages":{"state-init":"EXECUTE","domain-design":"EXECUTE","contract-design":"EXECUTE"}},
                "express":{"stages":{"state-init":"EXECUTE","domain-design":"EXECUTE","contract-design":"SKIP"}}}"#,
        )
        .expect("scope-grid.json");
        for scope in ["classic", "express"] {
            fs::write(
                scopes.join(format!("aidlc-{scope}.md")),
                format!("---\nname: {scope}\nkeywords: [\"{scope}-work\"]\n---\n\n# {scope}\n"),
            )
            .expect("scope identity");
        }
    }

    async fn invoke(&self, argv0: &str, args: &[&str]) -> aidlc::runtime::Completion {
        let mut owned: Vec<String> = args.iter().map(|a| (*a).to_string()).collect();
        owned.push("--project-dir".to_string());
        owned.push(self.project_dir().to_string_lossy().into_owned());
        aidlc::runtime::run(argv0, &owned, self.project_dir()).await
    }

    /// `next` を 1 回打って出た directive を JSON で返す。
    async fn next(&self, args: &[&str]) -> JsonValue {
        let mut argv = vec!["next"];
        argv.extend_from_slice(args);
        let completion = self.invoke("aidlc-orchestrate", &argv).await;
        assert_eq!(
            completion.code(),
            0,
            "ビジネス経路は exit 0: {completion:?}"
        );
        parse(
            completion
                .line()
                .unwrap_or_else(|| panic!("stdout に directive が要る: {completion:?}")),
        )
        .expect("directive は JSON")
    }

    async fn mint(&self, scope: &str) {
        let completion = self
            .invoke(
                "aidlc-utility",
                &["intent-create", "--scope", scope, "--label", "demo"],
            )
            .await;
        assert_eq!(completion.code(), 0, "鋳造は通る: {completion:?}");
    }

    /// カーソルのステージを 1 つ進める（ゲート付きなので承認の報告になる）。
    async fn report(&self, result: &str) {
        let completion = self
            .invoke("aidlc-orchestrate", &["report", "--result", result])
            .await;
        assert_eq!(completion.code(), 0, "報告は通る: {completion:?}");
    }
}

fn field(value: &JsonValue, key: &str) -> String {
    match value {
        JsonValue::Object(members) => match members.get(key) {
            Some(JsonValue::String(text)) => text.clone(),
            other => panic!("{key} は文字列であるべき: {other:?}"),
        },
        other => panic!("オブジェクトであるべき: {other:?}"),
    }
}

fn kind(value: &JsonValue) -> String {
    field(value, "kind")
}

// ---------------------------------------------------------------------------
// 前置ガード — リードモデルを 1 度も読まずに答えが決まる
// ---------------------------------------------------------------------------

/// フラグのパース失敗はそのまま逐語で中継する。
#[tokio::test]
async fn a_parse_error_is_relayed_verbatim() {
    let workspace = Workspace::create();

    let directive = workspace.next(&["--review"]).await;

    assert_eq!(kind(&directive), "error");
    assert_eq!(
        field(&directive, "message"),
        "--review requires <adversarial|advisory|none>."
    );
}

/// `--review` は他のモードと併用できない。
#[tokio::test]
async fn review_combined_with_another_mode_is_refused() {
    let workspace = Workspace::create();

    let directive = workspace.next(&["--review", "advisory", "--resume"]).await;

    assert_eq!(kind(&directive), "error");
    assert!(
        field(&directive, "message").starts_with("Cannot combine --review with read-only"),
        "{directive:?}"
    );
}

/// `--stage` と `--phase` の併用は拒否する。
#[tokio::test]
async fn stage_and_phase_together_are_refused() {
    let workspace = Workspace::create();

    let directive = workspace
        .next(&["--stage", "contract-design", "--phase", "inception"])
        .await;

    assert_eq!(kind(&directive), "error");
    assert_eq!(
        field(&directive, "message"),
        "Cannot use --stage and --phase together. Use one or the other."
    );
}

/// 前置ガードで止まるターンは、記録もストアも無くても答えを出す（I/O を起こさない証拠）。
#[tokio::test]
async fn a_pre_guard_turn_answers_without_any_read_model() {
    let workspace = Workspace {
        root: tempfile::tempdir().expect("一時ディレクトリ"),
    };

    let directive = workspace
        .next(&["--stage", "a", "--phase", "inception"])
        .await;

    assert_eq!(
        kind(&directive),
        "error",
        "定義もストアも無いのに答えが出る"
    );
}

// ---------------------------------------------------------------------------
// scope 解決ラダー
// ---------------------------------------------------------------------------

/// 無効な明示 `--scope` は、state が勝つ場合でも無条件に検証される。
#[tokio::test]
async fn an_invalid_explicit_scope_is_refused_even_when_state_wins() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;

    let directive = workspace.next(&["--scope", "nope"]).await;

    assert_eq!(kind(&directive), "error");
    assert_eq!(
        field(&directive, "message"),
        "Unknown scope \"nope\". Valid scopes: classic, express.",
        "有効 scope の一覧は行から綴り順で並べる"
    );
}

// ---------------------------------------------------------------------------
// 要求の形で決まる分岐
// ---------------------------------------------------------------------------

/// compose は composer のディスパッチを名指す。
#[tokio::test]
async fn compose_names_the_composer_dispatch() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;

    let directive = workspace.next(&["compose"]).await;

    assert_eq!(kind(&directive), "print");
    assert!(
        field(&directive, "message").starts_with("Dispatch the composer: run `"),
        "{directive:?}"
    );
}

/// compose と jump の併用は拒否する。
#[tokio::test]
async fn compose_with_a_jump_flag_is_refused() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;

    let directive = workspace
        .next(&["compose", "--stage", "contract-design"])
        .await;

    assert_eq!(kind(&directive), "error");
    assert!(
        field(&directive, "message").starts_with("Cannot combine compose with --stage/--phase."),
        "{directive:?}"
    );
}

/// `--new-intent` は空の記述を拒否する。
#[tokio::test]
async fn a_blank_new_intent_description_is_refused_verbatim() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;

    let directive = workspace.next(&["--new-intent"]).await;

    assert_eq!(kind(&directive), "error");
    assert_eq!(
        field(&directive, "message"),
        "`next --new-intent` requires a nonblank new-work description after the confirmed scope."
    );
}

/// `--new-intent` は鋳造コマンドとコスト節を名指し、セッションを畳めと言う。
#[tokio::test]
async fn new_intent_names_the_mint_command_with_its_cost_clause() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;

    let directive = workspace
        .next(&["--new-intent", "build the auth service"])
        .await;

    assert_eq!(kind(&directive), "print");
    let message = field(&directive, "message");
    assert!(
        message.contains("intent-create --scope classic --arguments='build the auth service'"),
        "{message}"
    );
    assert!(message.contains("stages,"), "コスト節が付く: {message}");
    assert!(
        message.contains("Then STOP, do NOT re-run `next` in this session."),
        "{message}"
    );
}

/// `--single` はステージを要る。
#[tokio::test]
async fn single_requires_a_stage() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;

    let directive = workspace.next(&["--single"]).await;

    assert_eq!(kind(&directive), "error");
    assert_eq!(
        field(&directive, "message"),
        "--single requires --stage <slug>."
    );
}

/// `--single` は孤立した run-stage を届ける（規則束があれば第 1 部から）。
#[tokio::test]
async fn single_emits_an_isolated_stage_delivery() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;

    let directive = workspace
        .next(&["--single", "--stage", "contract-design"])
        .await;

    assert_eq!(kind(&directive), "load-steering");
    assert_eq!(field(&directive, "stage"), "contract-design");
}

/// `--single` に未知のステージを渡すと逐語で拒否する。
#[tokio::test]
async fn single_with_an_unknown_stage_is_refused() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;

    let directive = workspace.next(&["--single", "--stage", "nowhere"]).await;

    assert_eq!(kind(&directive), "error");
    assert_eq!(
        field(&directive, "message"),
        "Unknown stage \"nowhere\". Run /aidlc --help for the full list."
    );
}

/// 有効で異なる scope は scope 変更の命令 1 本になる（修飾子も同じ命令へ載る）。
#[tokio::test]
async fn a_differing_valid_scope_names_one_scope_change_command() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;

    let directive = workspace
        .next(&["--scope", "express", "--depth", "standard"])
        .await;

    assert_eq!(kind(&directive), "print");
    let message = field(&directive, "message");
    assert!(
        message.contains("scope-change --scope express"),
        "{message}"
    );
    assert!(
        message.contains("--depth standard"),
        "修飾子も同じ 1 本へ: {message}"
    );
}

/// scope を変えない設定変更は config-change の命令になる。
#[tokio::test]
async fn a_configuration_change_without_a_scope_names_config_change() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;

    let directive = workspace.next(&["--test-strategy", "minimal"]).await;

    assert_eq!(kind(&directive), "print");
    let message = field(&directive, "message");
    assert!(message.contains("config-change"), "{message}");
    assert!(message.contains("--test-strategy minimal"), "{message}");
}

/// `--resume` は再開メニューを問う。
#[tokio::test]
async fn resume_asks_the_resume_menu() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;

    let directive = workspace.next(&["--resume"]).await;

    assert_eq!(kind(&directive), "ask");
    assert!(
        field(&directive, "question").starts_with("An existing workflow was found"),
        "{directive:?}"
    );
}

/// `--stage` は自分で跳ばず、跳ぶための命令を名指す。
#[tokio::test]
async fn a_stage_jump_names_the_resolve_command() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;

    let directive = workspace.next(&["--stage", "contract-design"]).await;

    assert_eq!(kind(&directive), "print");
    assert!(
        field(&directive, "message").contains("--stage contract-design"),
        "{directive:?}"
    );
}

/// initialization フェーズへは跳べない（行が拒否を運ぶ）。
#[tokio::test]
async fn a_jump_into_initialization_is_refused() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;

    let directive = workspace.next(&["--stage", "state-init"]).await;

    assert_eq!(kind(&directive), "error");
    assert!(
        field(&directive, "message").starts_with("Cannot jump to initialization stages."),
        "{directive:?}"
    );
}

/// 計画に無いステージへの `--stage` は未知として拒否する。
#[tokio::test]
async fn a_jump_to_an_unplanned_stage_is_refused() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;

    let directive = workspace.next(&["--stage", "nowhere"]).await;

    assert_eq!(kind(&directive), "error");
    assert_eq!(
        field(&directive, "message"),
        "Unknown stage \"nowhere\". Run /aidlc --help for the full list."
    );
}

/// `--phase` は そのフェーズの入口へ跳ぶ命令を名指す。
#[tokio::test]
async fn a_phase_jump_names_the_first_stage_of_that_phase() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;

    let directive = workspace.next(&["--phase", "inception"]).await;

    assert_eq!(kind(&directive), "print");
    assert!(
        field(&directive, "message").contains("--stage "),
        "{directive:?}"
    );
}

/// 未知のフェーズは逐語で拒否する。
#[tokio::test]
async fn an_unknown_phase_is_refused_verbatim() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;

    let directive = workspace.next(&["--phase", "nowhere"]).await;

    assert_eq!(kind(&directive), "error");
    assert_eq!(
        field(&directive, "message"),
        "Unknown phase \"nowhere\". Valid phases: initialization, ideation, inception, construction, operation."
    );
}

/// 稼働中の自由記述は「続きか、新規か、組み直しか」を問う。
#[tokio::test]
async fn free_text_on_a_running_workflow_asks_how_to_route_it() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;

    let directive = workspace.next(&["fix the login crash"]).await;

    assert_eq!(kind(&directive), "ask");
    assert_eq!(
        field(&directive, "question"),
        "Does this continue the active work, start separate new work, or re-shape the plan?"
    );
    assert_eq!(
        field(&directive, "new_work_description"),
        "fix the login crash"
    );
}

// ---------------------------------------------------------------------------
// ハッピーパス — 答えの綴りをそのまま描く
// ---------------------------------------------------------------------------

/// 素の `next` は次のステージの規則束を配る。
#[tokio::test]
async fn a_bare_next_delivers_the_next_stage() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;

    let directive = workspace.next(&[]).await;

    assert_eq!(kind(&directive), "load-steering");
    assert_eq!(field(&directive, "stage"), "domain-design");
}

/// 計画を走り切ると `done` で止まり、新規作業の出口案内が付く。
#[tokio::test]
async fn a_finished_plan_stops_with_the_completion_reason() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;
    workspace.report("approved").await;
    workspace.report("approved").await;

    let directive = workspace.next(&[]).await;

    assert_eq!(kind(&directive), "done");
    let reason = field(&directive, "reason");
    assert!(
        reason.starts_with("Workflow complete — no in-scope stage remains after"),
        "{reason}"
    );
    assert!(reason.contains("(scope: classic)"), "{reason}");
    assert!(
        reason.contains("If this input is genuinely NEW, unrelated work"),
        "新規作業の出口案内が付く: {reason}"
    );
}
