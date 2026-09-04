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

    /// 実行を実際に park する（`aidlc-orchestrate park` の実駆動）。
    async fn park(&self) {
        let completion = self.invoke("aidlc-orchestrate", &["park"]).await;
        assert_eq!(completion.code(), 0, "park は通る: {completion:?}");
    }

    /// カーソルのステージを 1 つ進める（ゲート付きなので承認の報告になる）。
    /// カーソルのゲートを 1 つ畳む。
    ///
    /// `[-]` のゲートは明示 `--stage` を要し（forward 表）、前進は人間の選択を要する
    /// （段 13）ので、どちらも添えて叩く。
    async fn report(&self, result: &str, stage: &str) {
        let completion = self
            .invoke(
                "aidlc-orchestrate",
                &[
                    "report",
                    "--result",
                    result,
                    "--user-input",
                    "A",
                    "--stage",
                    stage,
                ],
            )
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
    workspace.report("approved", "domain-design").await;
    workspace.report("approved", "contract-design").await;

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

// ---------------------------------------------------------------------------
// state なしの群 — まだ何も鋳造していないワークスペース
// ---------------------------------------------------------------------------

/// 何も名指されていない素の `next` は、始め方を 2 通り案内して止まる。
#[tokio::test]
async fn a_bare_next_on_a_fresh_workspace_reports_that_there_is_no_state() {
    let workspace = Workspace::create();

    let directive = workspace.next(&[]).await;

    assert_eq!(kind(&directive), "error");
    assert_eq!(
        field(&directive, "message"),
        "No workflow state found (no active intent). Start one by describing what to build \
(/aidlc \"build the auth service\") or by naming a scope (/aidlc --scope <scope>)."
    );
}

/// 明示 `--scope` は、その scope で鋳造する命令をコスト節つきで名指す。
#[tokio::test]
async fn an_explicit_scope_on_a_fresh_workspace_names_the_mint_command() {
    let workspace = Workspace::create();

    let directive = workspace.next(&["--scope", "express"]).await;

    assert_eq!(kind(&directive), "print");
    let message = field(&directive, "message");
    assert!(
        message.contains("intent-create --scope express"),
        "{message}"
    );
    assert!(
        message.contains("then re-run `next` to continue."),
        "同じセッションで続けてよい: {message}"
    );
}

/// 位置引数が scope 名そのものなら、自由記述ではなくその scope の鋳造になる。
#[tokio::test]
async fn a_positional_scope_name_on_a_fresh_workspace_names_the_mint_command() {
    let workspace = Workspace::create();

    let directive = workspace.next(&["express"]).await;

    assert_eq!(kind(&directive), "print");
    assert!(
        field(&directive, "message").contains("intent-create --scope express"),
        "{directive:?}"
    );
}

/// キーワードが当たれば、その scope でよいかをコスト節つきで確認する。
#[tokio::test]
async fn a_keyword_on_a_fresh_workspace_asks_to_confirm_the_inferred_scope() {
    let workspace = Workspace::create();

    let directive = workspace.next(&["classic-work"]).await;

    assert_eq!(kind(&directive), "ask");
    let question = field(&directive, "question");
    assert!(
        question.starts_with("This looks like \"classic\" work"),
        "{question}"
    );
    assert!(question.contains("stages,"), "コスト節が付く: {question}");
}

/// どの既製 scope も当たらなければ compose を提案し、既製の綴りを例に挙げる。
#[tokio::test]
async fn free_text_that_matches_no_scope_offers_compose() {
    let workspace = Workspace::create();

    let directive = workspace.next(&["frobnicate"]).await;

    assert_eq!(kind(&directive), "ask");
    let question = field(&directive, "question");
    assert!(
        question.starts_with("None of the ready-made plans is an obvious fit for: \"frobnicate\"."),
        "{question}"
    );
    assert!(
        question.contains("\"classic\""),
        "既製の綴りを挙げる: {question}"
    );
}

/// state が無い `--stage` は定義側の行を引くが、記録が無いので組み立てられない。
#[tokio::test]
async fn a_stateless_stage_jump_cannot_assemble_a_run_stage_without_a_record() {
    let workspace = Workspace::create();

    let directive = workspace.next(&["--stage", "contract-design"]).await;

    assert_eq!(kind(&directive), "error");
    assert_eq!(
        field(&directive, "message"),
        "No workspace record was resolved for run-stage assembly."
    );
}

/// state が無くても initialization フェーズへは跳べない。
#[tokio::test]
async fn a_stateless_jump_into_initialization_is_refused() {
    let workspace = Workspace::create();

    let directive = workspace.next(&["--stage", "state-init"]).await;

    assert_eq!(kind(&directive), "error");
    assert!(
        field(&directive, "message").starts_with("Cannot jump to initialization stages."),
        "{directive:?}"
    );
}

/// state が無い `--phase` は、そのフェーズの入口を定義から引く。
#[tokio::test]
async fn a_stateless_phase_jump_resolves_the_entry_stage_of_that_phase() {
    let workspace = Workspace::create();

    let directive = workspace.next(&["--phase", "inception"]).await;

    assert_eq!(
        kind(&directive),
        "error",
        "入口は引けるが記録が無いので組み立てられない: {directive:?}"
    );
    assert_eq!(
        field(&directive, "message"),
        "No workspace record was resolved for run-stage assembly."
    );
}

/// in-scope のステージが 1 つも無いフェーズは、そのフェーズ名で拒む。
#[tokio::test]
async fn a_stateless_phase_jump_into_an_empty_phase_is_refused() {
    let workspace = Workspace::create();

    let directive = workspace.next(&["--phase", "operation"]).await;

    assert_eq!(kind(&directive), "error");
    assert_eq!(
        field(&directive, "message"),
        "No in-scope stage found for phase \"operation\"."
    );
}

/// state が無い `--single` も、記録が無ければ組み立てられない。
#[tokio::test]
async fn a_stateless_single_cannot_assemble_a_run_stage_without_a_record() {
    let workspace = Workspace::create();

    let directive = workspace
        .next(&["--single", "--stage", "contract-design"])
        .await;

    assert_eq!(kind(&directive), "error");
    assert_eq!(
        field(&directive, "message"),
        "No workspace record was resolved for run-stage assembly."
    );
}

// ---------------------------------------------------------------------------
// `continue` — 連鎖の続きは token が運ぶ鍵だけで決まる
// ---------------------------------------------------------------------------

impl Workspace {
    /// `continue` を 1 回打って出た directive を JSON で返す。
    async fn resume(&self, token: &str) -> JsonValue {
        let completion = self.invoke("aidlc-orchestrate", &["continue", token]).await;
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
}

/// 連鎖の第 1 部が渡す token で `continue` を打つと、台帳を添えた終端 run-stage になる。
#[tokio::test]
async fn a_continue_with_the_delivered_token_closes_the_chain() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;
    let first = workspace.next(&[]).await;
    assert_eq!(kind(&first), "load-steering");
    let token = field(&first, "continue_token");

    let directive = workspace.resume(&token).await;

    assert_eq!(kind(&directive), "run-stage");
    assert_eq!(field(&directive, "stage"), "domain-design");
}

/// 開封できない token は原因を区別せず fail-closed の逐語になる。
#[tokio::test]
async fn a_continue_with_an_unopenable_token_fails_closed() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;

    let directive = workspace.resume("not-a-token").await;

    assert_eq!(kind(&directive), "error");
    assert_eq!(
        field(&directive, "message"),
        "Invalid steering continuation token: this stage's rules cannot be loaded from where \
they left off. Run a fresh `next` to restart delivery from part 1."
    );
}

// ---------------------------------------------------------------------------
// 引けない媒体・壊れた投影 — 実 CLI からは作れない状態を、ストアを直接いじって作る
//
// リードモデルは RMU が書いた正しいものしか存在しない。ここだけはストアを直接壊し、
// コントローラが「行が無い」と「引けない」を混ぜないことを固定する（park している答えは
// `park` の実駆動で作れるようになった — b45）。
// ---------------------------------------------------------------------------

impl Workspace {
    fn store_path(&self) -> PathBuf {
        self.path("aidlc/spaces/default/intents/.aidlc-store.sqlite")
    }

    /// ストアの中身を SQLite でないバイト列に置き換える (開けるが引けない状態)。
    fn break_store(&self) {
        fs::write(self.store_path(), b"not a sqlite database at all").expect("ストア");
    }

    /// ストアの置き場をディレクトリで塞ぐ (開くことすらできない状態)。
    fn block_store(&self) {
        fs::remove_file(self.store_path()).expect("ストア");
        fs::create_dir(self.store_path()).expect("塞ぐ");
    }
}

/// 空間名として成立しない active-space は、既定へ落とさず直し方を名指す。
#[tokio::test]
async fn an_invalid_active_space_names_the_cursor_file_to_fix() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;
    // record は解決できるが空間名が通らない状態にする（定義の下準備ではなく、引当の口を
    // 開く段で止まることを見るため）。
    let escaped = workspace.path("aidlc/escape/intents");
    fs::create_dir_all(&escaped).expect("escaped intents");
    fs::write(escaped.join("active-intent"), "260904-demo-abcd1234\n").expect("カーソル");
    fs::write(workspace.path("aidlc/active-space"), "../escape\n").expect("space カーソル");

    let directive = workspace.next(&[]).await;

    assert_eq!(kind(&directive), "error");
    let message = field(&directive, "message");
    assert!(
        message.starts_with("The active space \"../escape\" is not a valid space name."),
        "{message}"
    );
    assert!(message.contains("aidlc/active-space"), "{message}");
}

/// リードモデルを開けなければ、所在と分類を材料に「引けない」と答える。
#[tokio::test]
async fn a_store_that_cannot_be_opened_is_reported_with_its_path() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;
    workspace.block_store();

    let directive = workspace.next(&[]).await;

    assert_eq!(kind(&directive), "error");
    let message = field(&directive, "message");
    assert!(
        message.starts_with("Read model not readable at "),
        "{message}"
    );
    assert!(message.contains(".aidlc-store.sqlite"), "{message}");
}

/// 開けても引けなければ、不在ではなく読取失敗を答える。
#[tokio::test]
async fn a_store_that_opens_but_cannot_be_read_is_a_read_failure() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;
    workspace.break_store();

    let directive = workspace.next(&[]).await;

    assert_eq!(kind(&directive), "error");
    assert!(
        field(&directive, "message").starts_with("Read model not readable at "),
        "{directive:?}"
    );
}

/// 明示 `--scope` の検証も、scope カタログを引けなければそこで潰える。
#[tokio::test]
async fn an_unreadable_scope_catalog_stops_the_explicit_scope_check() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;
    workspace.break_store();

    let directive = workspace.next(&["--scope", "express"]).await;

    assert_eq!(kind(&directive), "error");
    assert!(
        field(&directive, "message").starts_with("Read model not readable at "),
        "{directive:?}"
    );
}

/// park している実行は、位置を名乗って止まる（`parked` directive はステージも運ぶ）。
///
/// 答えの綴りは注入せず、`park` を実駆動して投影させたものを読む（handoff-b44 の約束）。
#[tokio::test]
async fn a_parked_execution_stops_the_bare_next_at_its_stage() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;
    workspace.park().await;

    let directive = workspace.next(&[]).await;

    assert_eq!(kind(&directive), "parked");
    assert_eq!(field(&directive, "stage"), "domain-design");
    assert_eq!(
        field(&directive, "reason"),
        "Workflow parked at \"domain-design\". Resume with /aidlc --resume."
    );
}

/// park 位置の綴りが slug として読めなければ、park ではなく未知ステージとして拒む。
///
/// これは**壊れた投影の注入**である — 実駆動では作れない行なので、`park` してから
/// `read_next_answer` の綴りだけを直接壊す。
#[tokio::test]
async fn a_parked_answer_with_an_unreadable_slug_is_refused() {
    let workspace = Workspace::create();
    workspace.mint("classic").await;
    workspace.park().await;
    let connection = rusqlite::Connection::open(workspace.store_path()).expect("ストア");
    connection
        .execute(
            "UPDATE read_next_answer SET stage_slug = 'Not A Slug' WHERE request_kind = 'bare'",
            [],
        )
        .expect("綴りを壊す");

    let directive = workspace.next(&[]).await;

    assert_eq!(kind(&directive), "error");
    assert_eq!(
        field(&directive, "message"),
        "Unknown stage \"Not A Slug\". Run /aidlc --help for the full list."
    );
}

/// 定義の下準備は 2 回目以降なにも書かない — 走査済み位置に新しいイベントが無ければ
/// チェックポイントをそのまま返す（冪等）。
#[tokio::test]
async fn the_first_next_prepares_the_definition_only_once() {
    let workspace = Workspace::create();

    let first = workspace.next(&[]).await;
    let second = workspace.next(&[]).await;

    assert_eq!(kind(&first), "error", "state はまだ無い");
    assert_eq!(
        field(&second, "message"),
        field(&first, "message"),
        "2 回目も同じ答えになる"
    );
}
