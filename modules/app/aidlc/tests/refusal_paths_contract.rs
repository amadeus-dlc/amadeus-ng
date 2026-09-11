//! 公開 CLI の拒否経路と補助動詞を、配布定義（固定本家 2.7.1）の上で 1 経路ずつ固定する。
//!
//! 対象は「不正な引数を拒む」「壊れた前提を成功に丸めない」「文言が本家逐語である」という
//! 契約であり、幸福経路は既存の契約テストが持つ。stdin を要するフックだけ子プロセスで打ち、
//! それ以外は `aidlc::runtime::run` を同一プロセスで呼ぶ（起動名で面を選ぶ）。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use aidlc::runtime::Completion;

struct Workspace {
    temp: tempfile::TempDir,
}

impl Workspace {
    /// 配布定義 3 入力と bugfix / classic の scope 識別、memory 層を持つ空のワークスペース。
    /// `src/lib.rs` を 1 つ置いて Brownfield にする（reverse-engineering が最初の run-stage）。
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        let data = root.join(".claude/tools/data");
        fs::create_dir_all(&data).unwrap();
        let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        for name in ["stage-graph.json", "scope-grid.json", "harness.json"] {
            fs::copy(
                repository
                    .join("tests/golden/upstream-a277af21/data")
                    .join(name),
                data.join(name),
            )
            .unwrap();
        }
        fs::create_dir_all(root.join(".claude/scopes")).unwrap();
        for scope in ["bugfix", "classic"] {
            fs::copy(
                repository.join(format!(".claude/scopes/aidlc-{scope}.md")),
                root.join(format!(".claude/scopes/aidlc-{scope}.md")),
            )
            .unwrap();
        }
        fs::create_dir_all(root.join("aidlc/spaces/default/intents")).unwrap();
        let memory = root.join("aidlc/spaces/default/memory");
        fs::create_dir_all(&memory).unwrap();
        fs::write(
            memory.join("org.md"),
            "# Org\n\n## Way of Working\n\n規則。\n",
        )
        .unwrap();
        fs::write(
            memory.join("project.md"),
            "# Project-Level Rules\n\n## Forbidden\n\n## Mandated\n\n## Corrections\n",
        )
        .unwrap();
        fs::write(
            memory.join("team.md"),
            "# Team-Level Rules\n\n## Way of Working\n\n## Corrections\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn smoke() {}\n").unwrap();
        Self { temp }
    }

    fn root(&self) -> PathBuf {
        self.temp.path().join("workspace")
    }

    fn intents(&self) -> PathBuf {
        self.root().join("aidlc/spaces/default/intents")
    }

    fn record(&self) -> PathBuf {
        self.intents().join(
            fs::read_to_string(self.intents().join("active-intent"))
                .unwrap()
                .trim(),
        )
    }

    fn record_name(&self) -> String {
        self.record()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    }

    fn state(&self) -> String {
        fs::read_to_string(self.record().join("aidlc-state.md")).unwrap()
    }

    fn current_stage(&self) -> String {
        self.state()
            .lines()
            .find_map(|line| line.strip_prefix("- **Current Stage**:"))
            .unwrap()
            .trim()
            .to_string()
    }

    /// 起動名を選んで同一プロセスで 1 回打つ（`--project-dir` を添える）。
    async fn invoke(&self, argv0: &str, args: &[&str]) -> Completion {
        let mut owned: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
        owned.push("--project-dir".to_string());
        owned.push(self.root().to_string_lossy().into_owned());
        aidlc::runtime::run(argv0, &owned, &self.root()).await
    }

    async fn mint(&self, scope: &str, label: &str) {
        let completion = self
            .invoke(
                "aidlc-utility",
                &[
                    "intent-create",
                    "--scope",
                    scope,
                    "--label",
                    label,
                    "--arguments",
                    "Fix one small defect",
                ],
            )
            .await;
        assert_eq!(completion.code(), 0, "鋳造は通る: {completion:?}");
    }

    /// stdin を要するフックだけ子プロセスで打つ。
    fn hook(&self, name: &str, input: &str, environment: &[(&str, &str)]) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .args(["hook", name])
            .current_dir(self.root())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.temp.path().join("home"))
            .env("PATH", "/usr/bin:/bin")
            .env("AIDLC_DISABLE_USAGE_TRACKING", "1")
            .envs(environment.iter().copied())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    }
}

/// `aidlc-log` / `aidlc-jump` 面の拒否は本家 `emitError` の `{"error":…}` 1 行である。
fn refused_error(completion: &Completion) -> String {
    assert_eq!(completion.code(), 1, "{completion:?}");
    assert_eq!(completion.line(), None, "{completion:?}");
    let value: serde_json::Value = serde_json::from_str(completion.diagnostic().unwrap()).unwrap();
    value.get("error").unwrap().as_str().unwrap().to_string()
}

/// 平文 stderr の拒否（learnings / utility / orchestrate の自己防衛）。
fn refused_plain(completion: &Completion) -> String {
    assert_eq!(completion.code(), 1, "{completion:?}");
    assert_eq!(completion.line(), None, "{completion:?}");
    completion.diagnostic().unwrap().to_string()
}

fn emitted_json(completion: &Completion) -> serde_json::Value {
    assert_eq!(completion.code(), 0, "{completion:?}");
    serde_json::from_str(completion.line().unwrap()).unwrap()
}

// ---------------------------------------------------------------------------
// aidlc-learnings — 補助動詞と記録が無いときの拒否
// ---------------------------------------------------------------------------

#[tokio::test]
async fn learnings_help_is_terminal_and_unknown_verbs_point_at_it() {
    let workspace = Workspace::new();
    let help = workspace.invoke("aidlc-learnings", &["--help"]).await;
    assert_eq!(help.code(), 0);
    assert_eq!(help.line(), Some(aidlc::wording::LEARNINGS_HELP));
    let short = workspace.invoke("aidlc-learnings", &["-h"]).await;
    assert_eq!(short.line(), help.line());

    let unknown = workspace.invoke("aidlc-learnings", &["frobnicate"]).await;
    assert_eq!(
        refused_plain(&unknown),
        "Unknown subcommand: frobnicate. Run aidlc-learnings.ts --help for usage."
    );
    let none = workspace.invoke("aidlc-learnings", &[]).await;
    assert_eq!(
        refused_plain(&none),
        "Unknown subcommand: (none). Run aidlc-learnings.ts --help for usage."
    );
}

#[tokio::test]
async fn learnings_surface_distinguishes_no_record_from_an_ambiguous_cursor() {
    let workspace = Workspace::new();
    let usage = workspace.invoke("aidlc-learnings", &["surface"]).await;
    assert_eq!(
        refused_plain(&usage),
        aidlc::wording::LEARNINGS_SURFACE_USAGE
    );
    let none = workspace
        .invoke("aidlc-learnings", &["surface", "--slug", "intent-capture"])
        .await;
    assert_eq!(
        refused_plain(&none),
        aidlc::wording::LEARNINGS_WITHOUT_INTENT
    );
    // 記録ディレクトリが 2 つあってカーソルが無い — 本家 `:226-231` の曖昧拒否。
    for name in ["260101-one-aaaaaaaa", "260102-two-bbbbbbbb"] {
        fs::create_dir_all(workspace.intents().join(name)).unwrap();
    }
    let ambiguous = workspace
        .invoke("aidlc-learnings", &["surface", "--slug", "intent-capture"])
        .await;
    assert_eq!(
        refused_plain(&ambiguous),
        aidlc::wording::learnings_ambiguous_intent("default")
    );
}

#[tokio::test]
async fn learnings_persist_refuses_missing_space_intent_and_method_files() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "learn").await;
    let usage = workspace.invoke("aidlc-learnings", &["persist"]).await;
    assert_eq!(
        refused_plain(&usage),
        aidlc::wording::LEARNINGS_PERSIST_USAGE
    );
    let stage = workspace.current_stage();
    let selections = workspace.root().join("selections.json");
    let selections_arg = selections.to_string_lossy().into_owned();
    let write = |space: &str, intent: &str| {
        fs::write(
            &selections,
            format!(
                r#"{{"stage_slug":"{stage}","space":"{space}","intent":"{intent}","selections":[]}}"#
            ),
        )
        .unwrap();
    };
    write("nowhere", &workspace.record_name());
    let missing_space = workspace
        .invoke(
            "aidlc-learnings",
            &["persist", "--selections-json", &selections_arg],
        )
        .await;
    assert_eq!(
        refused_plain(&missing_space),
        aidlc::wording::learnings_missing_space("nowhere")
    );

    write("default", "260101-gone-cccccccc");
    let missing_intent = workspace
        .invoke(
            "aidlc-learnings",
            &["persist", "--selections-json", &selections_arg],
        )
        .await;
    assert_eq!(
        refused_plain(&missing_intent),
        aidlc::wording::learnings_missing_intent("260101-gone-cccccccc", "default")
    );

    write("default", &workspace.record_name());
    let team = workspace.root().join("aidlc/spaces/default/memory/team.md");
    fs::remove_file(&team).unwrap();
    let missing_method = workspace
        .invoke(
            "aidlc-learnings",
            &["persist", "--selections-json", &selections_arg],
        )
        .await;
    assert_eq!(
        refused_plain(&missing_method),
        aidlc::wording::learnings_method_files_missing(&team.to_string_lossy())
    );
}

// ---------------------------------------------------------------------------
// aidlc-log answer / decision — 引数と前提の拒否
// ---------------------------------------------------------------------------

#[tokio::test]
async fn log_answer_refuses_malformed_arguments_before_touching_any_record() {
    let workspace = Workspace::new();
    let missing_value = workspace
        .invoke("aidlc-log", &["answer", "--stage", "x", "--details"])
        .await;
    assert_eq!(refused_error(&missing_value), "Missing value for --details");

    let no_details = workspace
        .invoke("aidlc-log", &["answer", "--stage", "intent-capture"])
        .await;
    assert_eq!(refused_error(&no_details), "Missing --details <text>");

    let unknown_checkpoint = workspace
        .invoke(
            "aidlc-log",
            &[
                "answer",
                "--stage",
                "intent-capture",
                "--details",
                "A",
                "--checkpoint",
                "bogus",
            ],
        )
        .await;
    assert_eq!(
        refused_error(&unknown_checkpoint),
        "Unknown --checkpoint \"bogus\". Accepted: summary-confirmation, plan-approval"
    );

    let bad_choice = workspace
        .invoke(
            "aidlc-log",
            &[
                "answer",
                "--stage",
                "intent-capture",
                "--details",
                "Maybe later",
                "--checkpoint",
                "summary-confirmation",
            ],
        )
        .await;
    assert_eq!(
        refused_error(&bad_choice),
        "Cannot record the summary choice because reply \"Maybe later\" did not match an offered option. Present \"Looks correct\" and \"Request changes\". Re-present those choices and wait for the human to choose one."
    );

    for protected in ["unit", "single"] {
        let mut args = vec!["answer", "--stage", "intent-capture", "--details", "A"];
        let flag = format!("--{protected}");
        args.push(&flag);
        if protected == "unit" {
            args.push("u1");
        }
        let refused = workspace.invoke("aidlc-log", &args).await;
        assert_eq!(
            refused_error(&refused),
            format!("Cannot record answer: --{protected} is not wired in this build.")
        );
    }

    let no_execution = workspace
        .invoke(
            "aidlc-log",
            &["answer", "--stage", "intent-capture", "--details", "A"],
        )
        .await;
    assert_eq!(
        refused_error(&no_execution),
        "Cannot record answer without an active execution."
    );
    assert!(
        !workspace.intents().join(".aidlc-store.sqlite").exists(),
        "拒否はストアを作らない"
    );
}

#[tokio::test]
async fn log_answer_plan_approval_requires_one_target_and_a_known_choice() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "plan").await;
    let bad_choice = workspace
        .invoke(
            "aidlc-log",
            &[
                "answer",
                "--stage",
                "code-generation",
                "--details",
                "Whatever",
                "--checkpoint",
                "plan-approval",
                "--stage-level",
            ],
        )
        .await;
    assert_eq!(
        refused_error(&bad_choice),
        "Plan Approval requires Approve Plan or Request Changes"
    );
    let no_target = workspace
        .invoke(
            "aidlc-log",
            &[
                "answer",
                "--stage",
                "code-generation",
                "--details",
                "Request Changes",
                "--checkpoint",
                "plan-approval",
            ],
        )
        .await;
    assert_eq!(
        refused_error(&no_target),
        "Plan Approval requires exactly one of --unit <unit> or --stage-level"
    );
    let both = workspace
        .invoke(
            "aidlc-log",
            &[
                "answer",
                "--stage",
                "code-generation",
                "--details",
                "Approve Plan",
                "--checkpoint",
                "plan-approval",
                "--unit",
                "u1-demo",
                "--stage-level",
            ],
        )
        .await;
    assert_eq!(
        refused_error(&both),
        "Plan Approval requires exactly one of --unit <unit> or --stage-level"
    );
    // --unit だけなら対象は組めるが、質問文書が無いので読取で断られる（成功に丸めない）。
    let unit_only = workspace
        .invoke(
            "aidlc-log",
            &[
                "answer",
                "--stage",
                "code-generation",
                "--details",
                "Request Changes",
                "--checkpoint",
                "plan-approval",
                "--unit",
                "u1-demo",
            ],
        )
        .await;
    assert_eq!(unit_only.code(), 1, "{unit_only:?}");
}

#[tokio::test]
async fn log_decision_refuses_malformed_arguments_and_plan_targets() {
    let workspace = Workspace::new();
    let missing_value = workspace
        .invoke("aidlc-log", &["decision", "--stage"])
        .await;
    assert_eq!(refused_error(&missing_value), "Missing value for --stage");

    let unknown_checkpoint = workspace
        .invoke(
            "aidlc-log",
            &[
                "decision",
                "--stage",
                "intent-capture",
                "--decision",
                "Pick one",
                "--checkpoint",
                "bogus",
            ],
        )
        .await;
    assert_eq!(
        refused_error(&unknown_checkpoint),
        "Unknown --checkpoint \"bogus\". Accepted: summary-confirmation, plan-approval"
    );

    for protected in ["unit", "single"] {
        let mut args = vec![
            "decision",
            "--stage",
            "intent-capture",
            "--decision",
            "Pick",
        ];
        let flag = format!("--{protected}");
        args.push(&flag);
        if protected == "unit" {
            args.push("u1");
        }
        let refused = workspace.invoke("aidlc-log", &args).await;
        assert_eq!(
            refused_error(&refused),
            format!("Cannot record decision: --{protected} is not wired in this build.")
        );
    }

    let no_execution = workspace
        .invoke(
            "aidlc-log",
            &[
                "decision",
                "--stage",
                "intent-capture",
                "--decision",
                "Pick",
            ],
        )
        .await;
    assert_eq!(
        refused_error(&no_execution),
        "Cannot record decision without an active execution."
    );

    workspace.mint("bugfix", "decide").await;
    let no_target = workspace
        .invoke(
            "aidlc-log",
            &[
                "decision",
                "--stage",
                "code-generation",
                "--decision",
                "Approve the plan?",
                "--checkpoint",
                "plan-approval",
            ],
        )
        .await;
    assert_eq!(
        refused_error(&no_target),
        "Plan Approval requires exactly one of --unit <unit> or --stage-level"
    );
    let unit_target = workspace
        .invoke(
            "aidlc-log",
            &[
                "decision",
                "--stage",
                "code-generation",
                "--decision",
                "Approve the plan?",
                "--checkpoint",
                "plan-approval",
                "--unit",
                "u1-demo",
                "--rationale",
                "because",
            ],
        )
        .await;
    assert_eq!(unit_target.code(), 1, "{unit_target:?}");
}

#[tokio::test]
async fn a_summary_questions_file_must_live_inside_the_record_and_parse() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "summary").await;
    let stage = workspace.current_stage();
    let outside = workspace.root().join("outside-questions.md");
    fs::write(&outside, "# Q\n").unwrap();
    let refused = workspace
        .invoke(
            "aidlc-log",
            &[
                "decision",
                "--stage",
                &stage,
                "--decision",
                "Summary",
                "--checkpoint",
                "summary-confirmation",
                "--questions-file",
                "outside-questions.md",
            ],
        )
        .await;
    assert_eq!(
        refused_error(&refused),
        "Summary confirmation questions file must be inside the active intent record: outside-questions.md"
    );

    let relative = format!(
        "aidlc/spaces/default/intents/{}/inception/{stage}/{stage}-questions.md",
        workspace.record_name()
    );
    let absent = workspace
        .invoke(
            "aidlc-log",
            &[
                "decision",
                "--stage",
                &stage,
                "--decision",
                "Summary",
                "--checkpoint",
                "summary-confirmation",
                "--questions-file",
                &relative,
            ],
        )
        .await;
    assert_eq!(
        refused_error(&absent),
        format!("Summary confirmation questions file does not exist: {relative}")
    );

    let path = workspace.root().join(&relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "# Questions\n\nno summary section here\n").unwrap();
    let invalid = workspace
        .invoke(
            "aidlc-log",
            &[
                "decision",
                "--stage",
                &stage,
                "--decision",
                "Summary",
                "--checkpoint",
                "summary-confirmation",
                "--questions-file",
                &relative,
            ],
        )
        .await;
    let message = refused_error(&invalid);
    assert!(
        message.starts_with(&format!(
            "Summary confirmation questions file {relative} is invalid: "
        )) || message.starts_with(&format!(
            "Summary confirmation section in {relative} must contain exactly one `[Answer]:` line"
        )),
        "{message}"
    );

    // 回答側は `[Answer]:` の値が回答と一致しなければならない。
    fs::write(
        &path,
        "# Questions\n\n## Consolidated Summary Confirmation\nLooks correct / Request changes\n[Answer]: Looks correct\n",
    )
    .unwrap();
    let mismatched = workspace
        .invoke(
            "aidlc-log",
            &[
                "answer",
                "--stage",
                &stage,
                "--details",
                "Request changes",
                "--checkpoint",
                "summary-confirmation",
                "--questions-file",
                &relative,
            ],
        )
        .await;
    assert_eq!(
        refused_error(&mismatched),
        format!(
            "Summary confirmation section in {relative} must contain exactly one `[Answer]:` line with Request changes before this command runs."
        )
    );
}

// ---------------------------------------------------------------------------
// aidlc-jump — 動詞・方向・resolve の使い方
// ---------------------------------------------------------------------------

#[tokio::test]
async fn jump_refuses_unknown_verbs_directions_and_resolve_without_a_target() {
    let workspace = Workspace::new();
    let unknown = workspace.invoke("aidlc-jump", &["frob"]).await;
    assert_eq!(
        refused_error(&unknown),
        "Unknown subcommand: frob. Valid: resolve, execute"
    );
    let none = workspace.invoke("aidlc-jump", &[]).await;
    assert_eq!(
        refused_error(&none),
        "Unknown subcommand: undefined. Valid: resolve, execute"
    );
    let direction = workspace
        .invoke(
            "aidlc-jump",
            &[
                "execute",
                "--target",
                "code-generation",
                "--direction",
                "sideways",
            ],
        )
        .await;
    assert_eq!(
        refused_error(&direction),
        "Invalid direction: sideways. Valid: forward, backward, redo"
    );
    let no_direction = workspace
        .invoke("aidlc-jump", &["execute", "--target", "code-generation"])
        .await;
    assert_eq!(
        refused_error(&no_direction),
        "Invalid direction: undefined. Valid: forward, backward, redo"
    );

    workspace.mint("bugfix", "jump").await;
    let usage = workspace.invoke("aidlc-jump", &["resolve"]).await;
    assert_eq!(
        refused_error(&usage),
        "Usage: resolve --stage <slug|#> or --phase <name|#> [--scope <scope>]"
    );
    let unknown_phase = workspace
        .invoke("aidlc-jump", &["resolve", "--phase", "Nowhere"])
        .await;
    assert_eq!(refused_error(&unknown_phase), "Unknown phase: Nowhere");
    let phase = workspace
        .invoke("aidlc-jump", &["resolve", "--phase", "Construction"])
        .await;
    let value = emitted_json(&phase);
    assert_eq!(value.get("direction").unwrap(), "forward");
    assert_eq!(
        value.get("current_slug").unwrap(),
        workspace.current_stage().as_str()
    );
    let unreachable = workspace
        .invoke("aidlc-jump", &["resolve", "--phase", "Initialization"])
        .await;
    assert!(
        refused_error(&unreachable).ends_with("cannot be reached from this workflow"),
        "{unreachable:?}"
    );
}

#[tokio::test]
async fn jump_execute_lists_codekb_repositories_for_reverse_engineering_artifacts() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "codekb").await;
    let codekb = workspace.root().join("aidlc/spaces/default/codekb");
    fs::create_dir_all(codekb.join("repo-a")).unwrap();
    fs::write(codekb.join("stray.txt"), "not a repository\n").unwrap();
    let output = workspace
        .invoke(
            "aidlc-jump",
            &[
                "execute",
                "--target",
                "requirements-analysis",
                "--direction",
                "forward",
            ],
        )
        .await;
    let value = emitted_json(&output);
    assert_eq!(value.get("direction").unwrap(), "forward", "{value}");
}

// ---------------------------------------------------------------------------
// aidlc-testing-posture — 動詞と対象指定
// ---------------------------------------------------------------------------

fn posture_error(completion: &Completion) -> String {
    refused_error(completion)
}

#[tokio::test]
async fn testing_posture_refuses_unknown_verbs_and_malformed_targets() {
    let workspace = Workspace::new();
    let none = workspace.invoke("aidlc-testing-posture", &[]).await;
    assert_eq!(
        posture_error(&none),
        "Unknown subcommand: (none). Valid: resolve, render, fingerprint, verify, begin"
    );
    let verify = workspace.invoke("aidlc-testing-posture", &["verify"]).await;
    assert_eq!(
        posture_error(&verify),
        "Testing Posture verify is not connected"
    );
    let neither = workspace
        .invoke("aidlc-testing-posture", &["fingerprint"])
        .await;
    assert_eq!(
        posture_error(&neither),
        "fingerprint requires exactly one of --unit <unit> or --stage-level"
    );
    let both = workspace
        .invoke(
            "aidlc-testing-posture",
            &["fingerprint", "--unit", "u1", "--stage-level"],
        )
        .await;
    assert_eq!(
        posture_error(&both),
        "fingerprint accepts exactly one of --unit <unit> or --stage-level"
    );
    let blank = workspace
        .invoke(
            "aidlc-testing-posture",
            &["fingerprint", "--unit", "--stage"],
        )
        .await;
    assert_eq!(
        posture_error(&blank),
        "fingerprint requires a non-blank --unit <unit>"
    );
    let no_record = workspace
        .invoke("aidlc-testing-posture", &["fingerprint", "--stage-level"])
        .await;
    assert_eq!(
        posture_error(&no_record),
        "Code Generation approval authority requires an active workflow state"
    );
    let unit_no_record = workspace
        .invoke("aidlc-testing-posture", &["begin", "--unit", "u1-demo"])
        .await;
    assert_eq!(unit_no_record.code(), 1, "{unit_no_record:?}");
}

// ---------------------------------------------------------------------------
// hooks — 未知のフック名
// ---------------------------------------------------------------------------

#[test]
fn an_unknown_hook_name_is_refused_before_reading_stdin_semantics() {
    let workspace = Workspace::new();
    let output = workspace.hook("no-such-hook", "{}", &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "Unknown hook: no-such-hook\n"
    );
}

// ---------------------------------------------------------------------------
// aidlc-log link — 引数と handoff の拒否
// ---------------------------------------------------------------------------

#[tokio::test]
async fn pipeline_link_refuses_a_lost_cursor_and_a_handoff_outside_the_record() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "link").await;
    let record = workspace.record();
    let handoff = record.join("inception/reverse-engineering/developer-scan.md");
    fs::create_dir_all(handoff.parent().unwrap()).unwrap();
    // handoff が FIFO なら読む前に断る（規則ファイル以外を読まない）。
    #[cfg(unix)]
    {
        let status = Command::new("mkfifo").arg(&handoff).status().unwrap();
        assert!(status.success());
        let relative = handoff
            .strip_prefix(workspace.root())
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let refused = workspace
            .invoke(
                "aidlc-log",
                &[
                    "link",
                    "--stage",
                    "reverse-engineering",
                    "--link",
                    "aidlc-developer-agent",
                    "--artifact",
                    &relative,
                ],
            )
            .await;
        let message = refused_error(&refused);
        assert!(
            message.contains(
                "reverse-engineering developer handoff is not a regular file (a special file)"
            ) && message.contains("a FIFO, socket, or device file can block forever"),
            "{message}"
        );
        fs::remove_file(&handoff).unwrap();
    }
    fs::write(&handoff, "## Developer Code Scan Results\n").unwrap();
    // `./` と `..` を含む綴りでも同じ handoff を名指す（`normalize`）。
    let twisted = format!(
        "./aidlc/spaces/default/intents/{}/inception/./reverse-engineering/../reverse-engineering/developer-scan.md",
        workspace.record_name()
    );
    let linked = workspace
        .invoke(
            "aidlc-log",
            &[
                "link",
                "--stage",
                "reverse-engineering",
                "--link",
                "aidlc-developer-agent",
                "--artifact",
                &twisted,
            ],
        )
        .await;
    let value = emitted_json(&linked);
    assert_eq!(value.get("emitted").unwrap(), "PIPELINE_LINK_COMPLETED");
    assert_eq!(value.get("repo"), None);

    // `--repo` は登録済みの repo 識別を要する — この intent には無いので断る。
    let with_repo = workspace
        .invoke(
            "aidlc-log",
            &[
                "link",
                "--stage",
                "reverse-engineering",
                "--link",
                "aidlc-developer-agent",
                "--repo",
                "alpha",
            ],
        )
        .await;
    assert_eq!(
        refused_error(&with_repo),
        "Cannot record pipeline link for \"reverse-engineering\": this intent has no registered repo identity; omit --repo."
    );

    // 実行カーソルが失われた記録では intent を解決できない。
    fs::remove_file(record.join(".aidlc-execution")).unwrap();
    let lost = workspace
        .invoke(
            "aidlc-log",
            &[
                "link",
                "--stage",
                "reverse-engineering",
                "--link",
                "aidlc-architect-agent",
            ],
        )
        .await;
    assert_eq!(
        refused_error(&lost),
        "Cannot resolve the active intent for pipeline link logging."
    );
    fs::write(record.join(".aidlc-execution"), "garbage\n").unwrap();
    let broken = workspace
        .invoke(
            "aidlc-log",
            &[
                "link",
                "--stage",
                "reverse-engineering",
                "--link",
                "aidlc-architect-agent",
            ],
        )
        .await;
    assert_eq!(broken.code(), 1, "{broken:?}");
}

// ---------------------------------------------------------------------------
// aidlc-log review — 受領証の順序と付録の鮮度
// ---------------------------------------------------------------------------

fn requirements_documents(record: &Path, reviewed: bool) {
    let directory = record.join("inception/requirements-analysis");
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("requirements-analysis-questions.md"),
        "# Questions\n\n## Q1\n修正対象は何か。\nA. 小さな不具合\nX. Other (please specify)\n[Answer]: A\n\n## Consolidated Summary Confirmation\nLooks correct / Request changes\n[Answer]: Looks correct\n",
    )
    .unwrap();
    let mut requirements = String::from(
        "# Requirements\n\n## Functional Requirements\nFR1: 小さな不具合を修正する。\n\n## Sources\n[Q1] 承認済み回答。\n\n## Assumptions & Open Questions\nNone.\n",
    );
    if reviewed {
        requirements.push_str(
            "\n## Review\n\n**Reviewer:** aidlc-product-lead-agent\n**Verdict:** READY\n**Iteration:** 1\n\n### Findings\nNone.\n",
        );
    }
    fs::write(directory.join("requirements.md"), requirements).unwrap();
}

async fn review(workspace: &Workspace, iteration: &str, verdict: Option<&str>) -> Completion {
    let mut args = vec![
        "review",
        "--stage",
        "requirements-analysis",
        "--reviewer",
        "aidlc-product-lead-agent",
        "--iteration",
        iteration,
    ];
    if let Some(verdict) = verdict {
        args.extend(["--verdict", verdict]);
    }
    workspace.invoke("aidlc-log", &args).await
}

/// 反駁 2 回まで許す合成定義（配布の scope はすべて advisory = 予算 1 なので、待ち行列の
/// 拒否は adversarial の合成定義でしか踏めない）。
fn synthetic_adversarial_workspace() -> Workspace {
    let workspace = Workspace::new();
    let data = workspace.root().join(".claude/tools/data");
    let node = |slug: &str, number: &str, name: &str, phase: &str, extra: &str| {
        format!(
            r#"{{"slug":"{slug}","number":"{number}","name":"{name}","phase":"{phase}",
                 "execution":"ALWAYS","mode":"inline","lead_agent":"orchestrator",
                 "scopes":["classic"]{extra}}}"#
        )
    };
    let reviewed = r#","reviewer":"aidlc-architecture-reviewer-agent","review_artifact":"domain-design","produces":["domain-design"],"review_class":"adversarial","reviewer_max_iterations":2"#;
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
    .unwrap();
    fs::write(
        data.join("scope-grid.json"),
        r#"{"classic":{"stages":{"state-init":"EXECUTE","domain-design":"EXECUTE","contract-design":"EXECUTE"}}}"#,
    )
    .unwrap();
    fs::write(
        workspace.root().join(".claude/scopes/aidlc-classic.md"),
        "---\nname: classic\nreview_cap: adversarial\n---\n\n# Classic\n",
    )
    .unwrap();
    workspace
}

#[tokio::test]
async fn a_second_review_iteration_waits_for_the_pending_verdict() {
    let workspace = synthetic_adversarial_workspace();
    workspace.mint("classic", "review").await;
    let document = workspace
        .record()
        .join("inception/domain-design/domain-design.md");
    fs::create_dir_all(document.parent().unwrap()).unwrap();
    fs::write(&document, "# Domain design\n").unwrap();
    let args = |iteration: &'static str| {
        vec![
            "review",
            "--stage",
            "domain-design",
            "--reviewer",
            "aidlc-architecture-reviewer-agent",
            "--iteration",
            iteration,
        ]
    };
    let first = workspace.invoke("aidlc-log", &args("1")).await;
    assert_eq!(first.code(), 0, "{first:?}");
    let second = workspace.invoke("aidlc-log", &args("2")).await;
    assert_eq!(
        refused_error(&second),
        "Cannot start another review for \"domain-design\" because iteration 1 is still waiting for a verdict. Record that verdict, or repeat the same iteration with --retry-pending if the reviewer did not run."
    );
}

#[tokio::test]
async fn a_review_appendix_that_predates_the_request_is_not_fresh_evidence() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "stale").await;
    // 依頼の前から同じ `## Review` 節が在る — 付録は依頼後に書き直されなければならない。
    requirements_documents(&workspace.record(), true);
    let requested = review(&workspace, "1", None).await;
    assert_eq!(requested.code(), 0, "{requested:?}");
    let completed = review(&workspace, "1", Some("READY")).await;
    assert_eq!(
        refused_error(&completed),
        "Refusing REVIEW_COMPLETED for \"requirements-analysis\": the review appendix still starts with the exact section that existed before REVIEW_REQUESTED iteration 1, so it is not fresh reviewer evidence. Appending prose does not make stale reviewer authority fresh. Have the reviewer remove the old section and write a new `## Review` section for this iteration, then record the verdict."
    );
}

// ---------------------------------------------------------------------------
// session-start — 再選択の案内・単位表示・未コンパイルの stage
// ---------------------------------------------------------------------------

const SESSION: &str = "11111111-2222-4333-8444-555555555555";

fn intent_uuid(workspace: &Workspace, record_name: &str) -> String {
    let rows: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(workspace.intents().join("intents.json")).unwrap(),
    )
    .unwrap();
    rows.as_array()
        .unwrap()
        .iter()
        .find(|row| row.get("dirName").and_then(serde_json::Value::as_str) == Some(record_name))
        .unwrap()
        .get("uuid")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string()
}

fn additional_context(output: &Output) -> String {
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    value
        .get("additionalContext")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn a_resumed_session_stamped_on_another_intent_is_offered_a_rebind() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "first").await;
    let first = workspace.record_name();
    workspace.mint("bugfix", "second").await;
    let second = workspace.record_name();
    assert_ne!(first, second);
    let sessions = workspace.root().join("aidlc/.aidlc-sessions");
    fs::create_dir_all(&sessions).unwrap();
    fs::write(
        sessions.join(SESSION),
        format!("{}\n", intent_uuid(&workspace, &first)),
    )
    .unwrap();
    let input = format!(r#"{{"source":"resume","session_id":"{SESSION}"}}"#);

    let resumed = workspace.hook("session-start", &input, &[]);
    let context = additional_context(&resumed);
    assert!(
        context.contains(
            "INTENT REBIND OFFER: This conversation is bound to first, but the shared cursor names second. Move the shared cursor back to first? [Y/n] - on Yes, run `/aidlc intent first`;"
        ),
        "{context}"
    );
    assert_eq!(
        fs::read_to_string(sessions.join(format!("{SESSION}.rebind-offer")))
            .unwrap()
            .trim(),
        format!("default/{first}->default/{second}")
    );
    assert_eq!(
        fs::read_to_string(sessions.join(SESSION)).unwrap().trim(),
        intent_uuid(&workspace, &first),
        "印は選ばれた intent に留まる"
    );

    // 同じ案内は 2 度出さない（署名が一致する間は沈黙）。
    let again = workspace.hook("session-start", &input, &[]);
    assert!(
        !additional_context(&again).contains("INTENT REBIND OFFER"),
        "{again:?}"
    );

    // 再選択の照会だけを求める観測は、案内が無ければ何も出さない。
    let probe = workspace.hook(
        "session-start",
        &format!(r#"{{"source":"resume","session_id":"{SESSION}","rebind_check":true}}"#),
        &[],
    );
    assert_eq!(probe.status.code(), Some(0), "{probe:?}");
    assert!(probe.stdout.is_empty(), "{probe:?}");

    // 案内の署名を消せば、照会は案内だけを返す。
    fs::remove_file(sessions.join(format!("{SESSION}.rebind-offer"))).unwrap();
    let probe = workspace.hook(
        "session-start",
        &format!(r#"{{"source":"resume","session_id":"{SESSION}","rebind_check":true}}"#),
        &[],
    );
    let context = additional_context(&probe);
    assert!(
        context.starts_with(&format!(
            "AIDLC Runtime Session: {SESSION}\nINTENT REBIND OFFER"
        )),
        "{context}"
    );
}

#[tokio::test]
async fn a_bound_session_in_another_space_is_told_to_switch_space_first() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "first").await;
    let first = workspace.record_name();
    workspace.mint("bugfix", "second").await;
    let second = workspace.record_name();
    // 別 space `other` に first の記録を複製し、その uuid で印を打つ。
    let other = workspace.root().join("aidlc/spaces/other");
    fs::create_dir_all(other.join("intents")).unwrap();
    copy_tree(
        &workspace.intents().join(&first),
        &other.join("intents").join(&first),
    );
    fs::write(
        other.join("intents/intents.json"),
        format!(
            r#"[{{"uuid":"other-{}","dirName":"{first}","slug":"first"}}]"#,
            intent_uuid(&workspace, &first)
        ),
    )
    .unwrap();
    let sessions = workspace.root().join("aidlc/.aidlc-sessions");
    fs::create_dir_all(&sessions).unwrap();
    fs::write(
        sessions.join(SESSION),
        format!("other-{}\n", intent_uuid(&workspace, &first)),
    )
    .unwrap();
    let resumed = workspace.hook(
        "session-start",
        &format!(r#"{{"source":"resume","session_id":"{SESSION}"}}"#),
        &[],
    );
    let context = additional_context(&resumed);
    assert!(
        context.contains(
            "on Yes, first run `/aidlc space other`; after it completes, run `/aidlc intent first`;"
        ),
        "{context}"
    );
    assert_eq!(
        fs::read_to_string(sessions.join(format!("{SESSION}.rebind-offer")))
            .unwrap()
            .trim(),
        format!("other/{first}->default/{second}")
    );
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[tokio::test]
async fn session_start_reports_the_active_unit_and_uncompiled_stage_files() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "unit").await;
    let state_path = workspace.record().join("aidlc-state.md");
    let mut state = fs::read_to_string(&state_path).unwrap();
    state.push_str(
        "- **Active Unit**: u1-demo\n- **Unit State**: paused\n- **Unit Pause Reason**: waiting on review\n- **Unit Next Action**: resume after approval\n",
    );
    fs::write(&state_path, state).unwrap();
    // 配布のステージ本体に、コンパイル済みグラフが知らない slug を 1 つ置く。
    let stages = workspace.root().join(".claude/aidlc-common/stages");
    fs::create_dir_all(stages.join("construction")).unwrap();
    fs::write(stages.join("construction/mystery-stage.md"), "# Mystery\n").unwrap();
    fs::write(stages.join("construction/code-generation.md"), "# Known\n").unwrap();
    let started = workspace.hook(
        "session-start",
        &format!(r#"{{"source":"startup","session_id":"{SESSION}"}}"#),
        &[],
    );
    let context = additional_context(&started);
    assert!(
        context.contains(
            "Active Unit: u1-demo (paused; reason: waiting on review; next: resume after approval)\n"
        ),
        "{context}"
    );
    assert!(context.contains("mystery-stage"), "{context}");
    assert!(!context.contains("code-generation.md"), "{context}");
}

#[tokio::test]
async fn a_session_start_without_a_state_file_only_names_the_runtime_session() {
    let workspace = Workspace::new();
    let started = workspace.hook(
        "session-start",
        &format!(r#"{{"source":"startup","session_id":"{SESSION}"}}"#),
        &[],
    );
    assert_eq!(
        additional_context(&started),
        format!(
            "AIDLC Runtime Session: {SESSION}\nUse this exact value for any Plan Approval --session argument in this conversation."
        )
    );
    let anonymous = workspace.hook("session-start", r#"{"source":"startup"}"#, &[]);
    assert_eq!(anonymous.status.code(), Some(0));
    assert!(anonymous.stdout.is_empty(), "{anonymous:?}");
}

// ---------------------------------------------------------------------------
// hooks — 状態同期・セッション終了・ハートビート
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sync_workflow_state_records_a_failed_status_update_without_stopping_the_turn() {
    let workspace = Workspace::new();
    let input = r#"{"tool_name":"TaskUpdate","tool_input":{"taskId":"1","status":"in_progress","activeForm":"Running [reverse-engineering]"}}"#;
    // 状態ファイルが無ければ沈黙。
    let cold = workspace.hook("sync-workflow-state", input, &[]);
    assert_eq!(cold.status.code(), Some(0), "{cold:?}");
    assert!(cold.stdout.is_empty() && cold.stderr.is_empty(), "{cold:?}");
    workspace.mint("bugfix", "sync").await;
    let warm = workspace.hook("sync-workflow-state", input, &[]);
    assert_eq!(warm.status.code(), Some(0), "{warm:?}");
    assert!(warm.stdout.is_empty(), "{warm:?}");
    // 壊れた入力は封筒にならず沈黙する。
    let junk = workspace.hook("sync-workflow-state", "not json", &[]);
    assert_eq!(junk.status.code(), Some(0), "{junk:?}");
    assert!(junk.stdout.is_empty() && junk.stderr.is_empty(), "{junk:?}");
}

#[tokio::test]
async fn hook_health_refuses_an_invalid_active_space_by_name() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "space").await;
    fs::write(
        workspace.root().join("aidlc/active-space"),
        "not a space name!\n",
    )
    .unwrap();
    let audited = workspace.hook(
        "write-audit-log",
        r#"{"tool_name":"Write","tool_input":{"file_path":"README.md"}}"#,
        &[],
    );
    assert_eq!(audited.status.code(), Some(1), "{audited:?}");
    assert_eq!(
        String::from_utf8_lossy(&audited.stderr),
        format!(
            "{}\n",
            aidlc::wording::invalid_active_space("not a space name!")
        )
    );
}

// ---------------------------------------------------------------------------
// 壊れた保存物 — 成功に丸めず、原因ごとの文言で断る
// ---------------------------------------------------------------------------

impl Workspace {
    /// `.aidlc-execution` を文法外の内容に差し替える（在るのに読めない）。
    fn corrupt_cursor(&self) {
        fs::write(self.record().join(".aidlc-execution"), "garbage\n").unwrap();
    }

    /// 真実記録のストアをディレクトリで塞ぐ（開けない）。
    fn block_store(&self) {
        let store = self.intents().join(".aidlc-store.sqlite");
        fs::remove_file(&store).unwrap();
        fs::create_dir(&store).unwrap();
    }

    /// 記録を文法外の space 名の下へ複製し、active-space をそこへ向ける。
    fn move_into_an_invalid_space(&self) -> String {
        let bad = "not a space!";
        let target = self.root().join("aidlc/spaces").join(bad).join("intents");
        copy_tree(&self.intents(), &target);
        fs::write(self.root().join("aidlc/active-space"), format!("{bad}\n")).unwrap();
        bad.to_string()
    }
}

#[tokio::test]
async fn an_unreadable_execution_cursor_is_never_mistaken_for_a_fresh_workspace() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "cursor").await;
    workspace.corrupt_cursor();
    let expected_prefix = "The execution cursor cannot be read (malformed execution cursor at ";
    let decision = workspace
        .invoke(
            "aidlc-log",
            &[
                "decision",
                "--stage",
                "reverse-engineering",
                "--decision",
                "Pick",
            ],
        )
        .await;
    assert!(
        refused_error(&decision).starts_with(expected_prefix),
        "{decision:?}"
    );
    let answer = workspace
        .invoke(
            "aidlc-log",
            &["answer", "--stage", "reverse-engineering", "--details", "A"],
        )
        .await;
    assert!(
        refused_error(&answer).starts_with(expected_prefix),
        "{answer:?}"
    );
    let review = workspace
        .invoke(
            "aidlc-log",
            &[
                "review",
                "--stage",
                "requirements-analysis",
                "--reviewer",
                "aidlc-product-lead-agent",
                "--iteration",
                "1",
            ],
        )
        .await;
    assert!(
        refused_error(&review).starts_with(expected_prefix),
        "{review:?}"
    );
    let autonomy = workspace
        .invoke("aidlc-bolt", &["set-autonomy", "--mode", "autonomous"])
        .await;
    assert!(
        refused_plain(&autonomy).starts_with(expected_prefix),
        "{autonomy:?}"
    );
    let jump = workspace
        .invoke(
            "aidlc-jump",
            &[
                "execute",
                "--target",
                "code-generation",
                "--direction",
                "forward",
            ],
        )
        .await;
    assert!(
        refused_error(&jump).starts_with("malformed execution cursor at "),
        "jump は原因をそのまま運ぶ: {jump:?}"
    );
    let report = workspace
        .invoke("aidlc-orchestrate", &["report", "--result", "completed"])
        .await;
    assert_eq!(report.code(), 0, "{report:?}");
    let directive = emitted_json(&report);
    assert_eq!(directive.get("kind").unwrap(), "error");
    assert!(
        directive
            .get("message")
            .unwrap()
            .as_str()
            .unwrap()
            .starts_with(expected_prefix),
        "{directive}"
    );
}

#[tokio::test]
async fn a_store_that_cannot_be_opened_stops_every_writer_by_name() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "store").await;
    workspace.block_store();
    let decision = workspace
        .invoke(
            "aidlc-log",
            &[
                "decision",
                "--stage",
                "reverse-engineering",
                "--decision",
                "Pick",
            ],
        )
        .await;
    assert!(
        refused_error(&decision).starts_with("Audit emission failed: "),
        "{decision:?}"
    );
    let answer = workspace
        .invoke(
            "aidlc-log",
            &["answer", "--stage", "reverse-engineering", "--details", "A"],
        )
        .await;
    assert!(
        refused_error(&answer).starts_with("Audit emission failed: "),
        "{answer:?}"
    );
    let review = workspace
        .invoke(
            "aidlc-log",
            &[
                "review",
                "--stage",
                "requirements-analysis",
                "--reviewer",
                "aidlc-product-lead-agent",
                "--iteration",
                "1",
            ],
        )
        .await;
    assert_eq!(
        refused_error(&review),
        aidlc::wording::orchestrate_failure("cannot open the event store")
    );
    let link = workspace
        .invoke(
            "aidlc-log",
            &[
                "link",
                "--stage",
                "reverse-engineering",
                "--link",
                "aidlc-architect-agent",
            ],
        )
        .await;
    assert_eq!(
        refused_error(&link),
        "Cannot open the pipeline link repositories."
    );
}

#[tokio::test]
async fn an_invalid_active_space_is_refused_before_any_store_is_touched() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "space").await;
    let bad = workspace.move_into_an_invalid_space();
    let expected = aidlc::wording::invalid_active_space(&bad);
    let decision = workspace
        .invoke(
            "aidlc-log",
            &[
                "decision",
                "--stage",
                "reverse-engineering",
                "--decision",
                "Pick",
            ],
        )
        .await;
    assert_eq!(refused_error(&decision), expected);
    let answer = workspace
        .invoke(
            "aidlc-log",
            &["answer", "--stage", "reverse-engineering", "--details", "A"],
        )
        .await;
    assert_eq!(refused_error(&answer), expected);
    let review = workspace
        .invoke(
            "aidlc-log",
            &[
                "review",
                "--stage",
                "requirements-analysis",
                "--reviewer",
                "aidlc-product-lead-agent",
                "--iteration",
                "1",
            ],
        )
        .await;
    assert_eq!(refused_error(&review), expected);
    let link = workspace
        .invoke(
            "aidlc-log",
            &[
                "link",
                "--stage",
                "reverse-engineering",
                "--link",
                "aidlc-architect-agent",
            ],
        )
        .await;
    assert_eq!(refused_error(&link), expected);
    let autonomy = workspace
        .invoke("aidlc-bolt", &["set-autonomy", "--mode", "autonomous"])
        .await;
    assert_eq!(refused_plain(&autonomy), expected);
    let jump = workspace
        .invoke(
            "aidlc-jump",
            &[
                "execute",
                "--target",
                "code-generation",
                "--direction",
                "forward",
            ],
        )
        .await;
    assert_eq!(refused_error(&jump), expected);
    let posture = workspace.invoke("aidlc-testing-posture", &["render"]).await;
    assert_eq!(posture_error(&posture), expected);
    let surface = workspace
        .invoke(
            "aidlc-learnings",
            &["surface", "--slug", &workspace_stage(&workspace, &bad)],
        )
        .await;
    assert_eq!(surface.code(), 1, "{surface:?}");
    let report = workspace
        .invoke("aidlc-orchestrate", &["report", "--result", "completed"])
        .await;
    assert_eq!(
        refused_plain(&report),
        format!("aidlc-orchestrate: {expected}"),
        "読取前の追いつきが space 名で止まる"
    );
}

fn workspace_stage(workspace: &Workspace, space: &str) -> String {
    let intents = workspace
        .root()
        .join("aidlc/spaces")
        .join(space)
        .join("intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    fs::read_to_string(record.join("aidlc-state.md"))
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix("- **Current Stage**:"))
        .unwrap()
        .trim()
        .to_string()
}

// ---------------------------------------------------------------------------
// next — 名詞トークンの終端案内と brownfield のコスト節
// ---------------------------------------------------------------------------

#[tokio::test]
async fn plugin_and_knowledge_nouns_are_terminal_utilities_not_workflow_work() {
    let workspace = Workspace::new();
    for tokens in [
        vec!["plugin", "list"],
        vec!["plugin-create", "my-plugin"],
        vec!["knowledge", "list"],
    ] {
        let mut args = vec!["next"];
        args.extend(tokens.iter().copied());
        let directive = emitted_json(&workspace.invoke("aidlc-orchestrate", &args).await);
        assert_eq!(directive.get("kind").unwrap(), "print", "{directive}");
        assert_eq!(
            directive.get("message").unwrap(),
            aidlc::wording::terminal_utility(&format!(
                "bun .claude/tools/aidlc-utility.ts {}",
                tokens.join(" ")
            ))
            .as_str()
        );
    }
    assert!(
        !workspace.intents().join(".aidlc-store.sqlite").exists(),
        "終端案内はワークフローを読まない"
    );
}

#[tokio::test]
async fn a_brownfield_workspace_prices_the_inferred_scope_with_reverse_engineering() {
    let workspace = Workspace::new();
    // 配布の bugfix はブロック列の `keywords:` (`- fix`) を持つ。配布ファイルどおりの形で
    // キーワード推論が効くこと (本家 `listField` と同じ受理範囲) をここで踏む。
    let scope = workspace.root().join(".claude/scopes/aidlc-bugfix.md");
    let raw = fs::read_to_string(&scope).unwrap();
    assert!(
        raw.contains("keywords:\n  - fix\n  - bug\n  - broken\n"),
        "配布ファイルの形が変わった: {raw}"
    );
    let directive = emitted_json(
        &workspace
            .invoke("aidlc-orchestrate", &["next", "fix", "the", "crash"])
            .await,
    );
    assert_eq!(directive.get("kind").unwrap(), "ask", "{directive}");
    let question = directive.get("question").unwrap().as_str().unwrap();
    assert!(
        question.starts_with("This looks like \"bugfix\" work"),
        "{question}"
    );
    // brownfield では reverse-engineering を畳まない名目コストが付く（greenfield より 1 段多い）。
    fs::remove_dir_all(workspace.root().join("src")).unwrap();
    let greenfield = emitted_json(
        &workspace
            .invoke("aidlc-orchestrate", &["next", "fix", "the", "crash"])
            .await,
    );
    let greenfield_question = greenfield.get("question").unwrap().as_str().unwrap();
    assert_ne!(question, greenfield_question);
    let stages = |text: &str| -> u32 {
        text.split(" - ")
            .nth(1)
            .and_then(|clause| clause.split(" of ").next())
            .and_then(|count| count.trim().parse().ok())
            .unwrap_or_else(|| panic!("コスト節が要る: {text}"))
    };
    assert_eq!(stages(question), stages(greenfield_question) + 1);
}

// ---------------------------------------------------------------------------
// next — 後続ステージの語り（リード帽と支援エージェント）
// ---------------------------------------------------------------------------

/// 3 段の合成定義: 2 段目はリードだけ、3 段目はリードと支援 2 名を宣言する。
fn synthetic_hatted_workspace() -> Workspace {
    let workspace = Workspace::new();
    let data = workspace.root().join(".claude/tools/data");
    let node = |slug: &str, number: &str, name: &str, phase: &str, extra: &str| {
        format!(
            r#"{{"slug":"{slug}","number":"{number}","name":"{name}","phase":"{phase}",
                 "execution":"ALWAYS","mode":"inline","scopes":["classic"]{extra}}}"#
        )
    };
    fs::write(
        data.join("stage-graph.json"),
        format!(
            "[{},{},{},{},{}]",
            node("state-init", "0.1", "State Init", "initialization", r#","lead_agent":"orchestrator""#),
            node("domain-design", "1.1", "Domain Design", "inception", r#","lead_agent":"orchestrator""#),
            node("contract-design", "1.2", "Contract Design", "inception", r#","lead_agent":"aidlc-architect-agent","produces":["contract.json"]"#),
            node(
                "units-generation",
                "1.3",
                "Units Generation",
                "inception",
                r#","lead_agent":"aidlc-architect-agent","support_agents":["aidlc-developer-agent","aidlc-quality-agent"]"#
            ),
            node(
                "delivery-planning",
                "1.4",
                "Delivery Planning",
                "inception",
                r#","lead_agent":"aidlc-delivery-agent","support_agents":["aidlc-product-agent"]"#
            ),
        ),
    )
    .unwrap();
    fs::write(
        data.join("scope-grid.json"),
        r#"{"classic":{"stages":{"state-init":"EXECUTE","domain-design":"EXECUTE","contract-design":"EXECUTE","units-generation":"EXECUTE","delivery-planning":"EXECUTE"}}}"#,
    )
    .unwrap();
    fs::write(
        workspace.root().join(".claude/scopes/aidlc-classic.md"),
        "---\nname: classic\n---\n\n# Classic\n",
    )
    .unwrap();
    workspace
}

impl Workspace {
    /// `next` → `continue <token>` で終端の run-stage を取り出す。
    async fn run_stage(&self) -> serde_json::Value {
        let first = emitted_json(&self.invoke("aidlc-orchestrate", &["next"]).await);
        let directive = if first.get("kind").unwrap() == "load-steering" {
            let token = first
                .get("continue_token")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();
            emitted_json(
                &self
                    .invoke("aidlc-orchestrate", &["continue", &token])
                    .await,
            )
        } else {
            first
        };
        assert_eq!(directive.get("kind").unwrap(), "run-stage", "{directive}");
        directive
    }

    async fn approve(&self, stage: &str) {
        let completion = self
            .invoke(
                "aidlc-orchestrate",
                &[
                    "report",
                    "--result",
                    "approved",
                    "--user-input",
                    "A",
                    "--stage",
                    stage,
                ],
            )
            .await;
        assert_eq!(completion.code(), 0, "{completion:?}");
    }
}

#[tokio::test]
async fn later_stages_narrate_the_lead_hat_and_the_supporting_roles() {
    let workspace = synthetic_hatted_workspace();
    workspace.mint("classic", "hats").await;
    let first = workspace.run_stage().await;
    assert_eq!(first.get("stage").unwrap(), "domain-design");
    assert!(
        first
            .get("narration")
            .unwrap()
            .as_str()
            .unwrap()
            .starts_with("Starting the classic plan"),
        "{first}"
    );
    workspace.approve("domain-design").await;
    let second = workspace.run_stage().await;
    assert_eq!(second.get("stage").unwrap(), "contract-design");
    assert_eq!(
        second.get("narration").unwrap(),
        "Now working on Contract Design, wearing the architect hat."
    );
    workspace.approve("contract-design").await;
    let third = workspace.run_stage().await;
    assert_eq!(third.get("stage").unwrap(), "units-generation");
    assert_eq!(
        third.get("narration").unwrap(),
        "Now working on Units Generation, wearing the architect hat, with the developer and quality engineer on hand."
    );
    workspace.approve("units-generation").await;
    let fourth = workspace.run_stage().await;
    assert_eq!(fourth.get("stage").unwrap(), "delivery-planning");
    assert_eq!(
        fourth.get("narration").unwrap(),
        "Now working on Delivery Planning, wearing the delivery lead hat, with the product manager on hand."
    );
}

// ---------------------------------------------------------------------------
// continue-workflow (Stop) — 規則配送の待ち・会話の切り出し・上限超えの指示
// ---------------------------------------------------------------------------

fn drops(workspace: &Workspace) -> String {
    fs::read_to_string(
        workspace
            .record()
            .join(".aidlc-hooks-health/continue-workflow.drops"),
    )
    .unwrap_or_default()
}

#[tokio::test]
async fn a_stop_before_the_rules_are_loaded_blocks_with_the_steering_instruction() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "stop").await;
    let output = workspace.hook("continue-workflow", r#"{"stop_hook_active":false}"#, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let answer: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(answer.get("decision").unwrap(), "block", "{answer}");
    let reason = answer.get("reason").unwrap().as_str().unwrap();
    assert!(
        reason.starts_with("The AIDLC workflow still has rules to load for \"reverse-engineering\". Preserve this step-two continuation command, but do not run it yet: `bun .claude/tools/aidlc-orchestrate.ts continue \""),
        "{reason}"
    );
    assert!(
        reason.ends_with("\n\n[{\"path\":\"aidlc/spaces/default/memory/org.md\",\"text\":\"# Org\\n\\n\"},{\"path\":\"aidlc/spaces/default/memory/org.md\",\"text\":\"## Way of Working\\n\\n規則。\\n\"}]"),
        "{reason}"
    );
}

#[tokio::test]
async fn a_conversational_turn_is_allowed_to_end_when_the_human_spoke_after_the_engine() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "talk").await;
    let record = workspace.record();
    fs::write(record.join(".aidlc-engine-touch"), "engine\n").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    fs::write(record.join(".aidlc-human-turn"), "human\n").unwrap();
    let output = workspace.hook("continue-workflow", r#"{"stop_hook_active":false}"#, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(
        drops(&workspace).contains("the ending turn was conversational"),
        "{}",
        drops(&workspace)
    );
    // 印がディレクトリなら会話とはみなさない（通常の停止制御へ戻る）。
    fs::remove_file(record.join(".aidlc-human-turn")).unwrap();
    fs::create_dir(record.join(".aidlc-human-turn")).unwrap();
    let output = workspace.hook("continue-workflow", r#"{"stop_hook_active":false}"#, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(!output.stdout.is_empty(), "{output:?}");
}

#[tokio::test]
async fn an_oversized_active_directive_is_not_read_as_a_resume_wait() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "big").await;
    let directive = workspace.record().join(".aidlc-active-directive.json");
    fs::write(&directive, "x".repeat(70_000)).unwrap();
    let output = workspace.hook("continue-workflow", r#"{"stop_hook_active":false}"#, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(
        !output.stdout.is_empty(),
        "上限超えの指示は待ちにならず通常の停止制御へ: {output:?}"
    );
}

// ---------------------------------------------------------------------------
// 共有承認ストアの初期化印 — 版違い・印なし・読めない印
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_shared_approval_marker_of_another_version_or_without_a_store_is_refused() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "marker").await;
    let aidlc = workspace.root().join("aidlc");
    let marker = aidlc.join(".aidlc-runtime.state.json");
    let audit_input = r#"{"tool_name":"Write","tool_input":{"file_path":"README.md"}}"#;
    // 版 2 の印は読まない。
    fs::write(&marker, r#"{"version":2,"phase":"ready"}"#).unwrap();
    let output = workspace.hook("write-audit-log", audit_input, &[]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Unsupported shared approval initialization marker\n"
    );
    // 壊れた印。
    fs::write(&marker, "{not json").unwrap();
    let output = workspace.hook("write-audit-log", audit_input, &[]);
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Invalid shared approval initialization marker\n"
    );
    // 印がディレクトリなら読取の失敗をそのまま伝える。
    fs::remove_file(&marker).unwrap();
    fs::create_dir(&marker).unwrap();
    let output = workspace.hook("write-audit-log", audit_input, &[]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Is a directory"),
        "{output:?}"
    );
    fs::remove_dir(&marker).unwrap();
    // 印が無いのにストアだけ在るのは初期化の途中ではなく喪失である。
    let store = aidlc.join(".aidlc-runtime.sqlite");
    fs::write(&store, "").unwrap();
    let output = workspace.hook("write-audit-log", audit_input, &[]);
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Shared approval store has no initialization marker\n"
    );
    // ready の印があるのにストアが無いのも喪失である。
    fs::remove_file(&store).unwrap();
    fs::write(&marker, r#"{"version":1,"phase":"ready"}"#).unwrap();
    let output = workspace.hook("write-audit-log", audit_input, &[]);
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Previously used shared approval store is missing\n"
    );
}

#[tokio::test]
async fn validate_state_skips_context_invalidation_until_the_shared_store_is_ready() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "compact").await;
    let marker = workspace.root().join("aidlc/.aidlc-runtime.state.json");
    let input = format!(r#"{{"session_id":"{SESSION}","trigger":"manual"}}"#);
    // 印が無い（承認ストア未使用）: 失効処理は何もせず、breadcrumb は書く。
    assert!(!marker.exists());
    let output = workspace.hook("validate-state", &input, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(workspace.record().join(".aidlc-recovery.md").exists());
    // 初期化途中の印: 承認の発行履歴は無いので失効処理は何もしない。
    fs::write(&marker, r#"{"version":1,"phase":"initializing"}"#).unwrap();
    let output = workspace.hook("validate-state", &input, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    // 版違いの印はハートビートの段で断る（breadcrumb も書かない）。
    fs::remove_file(workspace.record().join(".aidlc-recovery.md")).unwrap();
    fs::write(&marker, r#"{"version":2,"phase":"ready"}"#).unwrap();
    let output = workspace.hook("validate-state", &input, &[]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(!workspace.record().join(".aidlc-recovery.md").exists());
}

// ---------------------------------------------------------------------------
// aidlc-learnings — 状態ファイルと実行カーソルの破損
// ---------------------------------------------------------------------------

#[tokio::test]
async fn learnings_surface_needs_a_readable_state_with_a_current_stage() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "state").await;
    let stage = workspace.current_stage();
    let state_path = workspace.record().join("aidlc-state.md");
    let original = fs::read_to_string(&state_path).unwrap();
    let without_stage = original
        .lines()
        .filter(|line| !line.starts_with("- **Current Stage**:"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&state_path, without_stage).unwrap();
    let missing = workspace
        .invoke("aidlc-learnings", &["surface", "--slug", &stage])
        .await;
    assert_eq!(
        refused_plain(&missing),
        aidlc::wording::LEARNINGS_NO_CURRENT_STAGE
    );
    fs::write(&state_path, &original).unwrap();
    let mismatch = workspace
        .invoke("aidlc-learnings", &["surface", "--slug", "code-generation"])
        .await;
    assert_eq!(
        refused_plain(&mismatch),
        aidlc::wording::learnings_slug_is_not_current("code-generation", &stage)
    );
}

#[tokio::test]
async fn learnings_persist_refuses_a_record_whose_cursor_is_lost_or_broken() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "cursorless").await;
    let stage = workspace.current_stage();
    let selections = workspace.root().join("selections.json");
    fs::write(
        &selections,
        format!(
            r#"{{"stage_slug":"{stage}","space":"default","intent":"{}","selections":[]}}"#,
            workspace.record_name()
        ),
    )
    .unwrap();
    let selections_arg = selections.to_string_lossy().into_owned();
    let cursor = workspace.record().join(".aidlc-execution");
    let saved = fs::read(&cursor).unwrap();
    fs::remove_file(&cursor).unwrap();
    let lost = workspace
        .invoke(
            "aidlc-learnings",
            &["persist", "--selections-json", &selections_arg],
        )
        .await;
    assert_eq!(
        refused_plain(&lost),
        aidlc::wording::learnings_missing_intent(&workspace.record_name(), "default")
    );
    fs::write(&cursor, "garbage\n").unwrap();
    let broken = workspace
        .invoke(
            "aidlc-learnings",
            &["persist", "--selections-json", &selections_arg],
        )
        .await;
    assert!(
        refused_plain(&broken)
            .contains("The execution cursor cannot be read (malformed execution cursor at "),
        "読取前の追いつきがカーソルで止まる: {broken:?}"
    );
    fs::write(&cursor, saved).unwrap();
}

#[tokio::test]
async fn learnings_surface_reads_the_stage_row_of_a_hand_written_runtime_graph() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "graph").await;
    let stage = workspace.current_stage();
    let graph = workspace.record().join("runtime-graph.json");
    let args = ["surface", "--slug", stage.as_str()];
    let surface = || workspace.invoke("aidlc-learnings", &args);
    assert_eq!(
        refused_plain(&surface().await),
        aidlc::wording::learnings_runtime_graph_missing(&graph.to_string_lossy())
    );
    fs::write(&graph, "{broken").unwrap();
    assert!(
        refused_plain(&surface().await).starts_with("runtime-graph.json is malformed: "),
        "{:?}",
        surface().await
    );
    fs::write(&graph, r#"{"stages":[{"stage_slug":"other"}]}"#).unwrap();
    assert_eq!(
        refused_plain(&surface().await),
        aidlc::wording::learnings_stage_not_in_graph(&stage)
    );
    fs::write(
        &graph,
        format!(r#"{{"stages":[{{"stage_slug":"{stage}"}}]}}"#),
    )
    .unwrap();
    assert_eq!(
        refused_plain(&surface().await),
        aidlc::wording::learnings_stage_without_memory_path(&stage)
    );
    fs::write(
        &graph,
        format!(
            r#"{{"stages":[{{"stage_slug":"{stage}","memory_path":"aidlc/spaces/default/intents/{}/inception/{stage}/memory.md"}}]}}"#,
            workspace.record_name()
        ),
    )
    .unwrap();
    let surfaced = emitted_json(&surface().await);
    assert_eq!(surfaced.get("stage_slug").unwrap(), stage.as_str());
    assert_eq!(surfaced.get("phase").unwrap(), "inception");
    assert_eq!(surfaced.get("memory_entries_total").unwrap(), 0);
}

#[tokio::test]
async fn learnings_persist_refuses_a_candidate_id_that_breaks_the_audit_line() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "candidate").await;
    let stage = workspace.current_stage();
    let selections = workspace.root().join("selections.json");
    fs::write(
        &selections,
        format!(
            r#"{{"stage_slug":"{stage}","space":"default","intent":"{}","selections":[{{"candidate_id":"two\nlines","type":"learning","scope":"project","heading":"Mandated","text":"ALWAYS keep going.","source":"user_addition"}}]}}"#,
            workspace.record_name()
        ),
    )
    .unwrap();
    let selections_arg = selections.to_string_lossy().into_owned();
    let refused = workspace
        .invoke(
            "aidlc-learnings",
            &["persist", "--selections-json", &selections_arg],
        )
        .await;
    assert_eq!(
        refused_plain(&refused),
        aidlc::wording::learnings_selection_bad_candidate_id("two\nlines")
    );
}

// ---------------------------------------------------------------------------
// rebuild-stage-graph — 発火しない条件と日誌の走査
// ---------------------------------------------------------------------------

const COMPILE_INPUT: &str = r#"{"session_id":"11111111-2222-4333-8444-555555555555","tool_name":"Bash","tool_input":{"command":"bun .claude/tools/aidlc-orchestrate.ts report --result completed"},"tool_response":{"stdout":""}}"#;

#[tokio::test]
async fn rebuild_stage_graph_stays_silent_without_a_record_or_an_audit_and_skips_odd_journals() {
    let workspace = Workspace::new();
    // 記録が無い。
    let cold = workspace.hook("rebuild-stage-graph", COMPILE_INPUT, &[]);
    assert_eq!(cold.status.code(), Some(0), "{cold:?}");
    assert!(cold.stdout.is_empty() && cold.stderr.is_empty(), "{cold:?}");
    workspace.mint("bugfix", "graph").await;
    let record = workspace.record();
    // 監査シャードが読めない（台帳がファイル）。
    let audit = record.join("audit");
    let shards: Vec<PathBuf> = fs::read_dir(&audit)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    let saved: Vec<(PathBuf, Vec<u8>)> = shards
        .iter()
        .map(|path| (path.clone(), fs::read(path).unwrap()))
        .collect();
    fs::remove_dir_all(&audit).unwrap();
    fs::write(&audit, "not a directory\n").unwrap();
    let unreadable = workspace.hook("rebuild-stage-graph", COMPILE_INPUT, &[]);
    assert_eq!(unreadable.status.code(), Some(0), "{unreadable:?}");
    assert!(!record.join("runtime-graph.json").exists());
    fs::remove_file(&audit).unwrap();
    fs::create_dir(&audit).unwrap();
    // 監査が空。
    let empty = workspace.hook("rebuild-stage-graph", COMPILE_INPUT, &[]);
    assert_eq!(empty.status.code(), Some(0), "{empty:?}");
    assert!(!record.join("runtime-graph.json").exists());
    for (path, bytes) in &saved {
        fs::write(path, bytes).unwrap();
    }
    // 文法外の stage ディレクトリ名と日誌の無い stage は走査から外れる。
    fs::create_dir_all(record.join("inception/Not A Slug")).unwrap();
    fs::write(record.join("inception/Not A Slug/memory.md"), "# odd\n").unwrap();
    fs::create_dir_all(record.join("inception/user-stories")).unwrap();
    fs::create_dir_all(record.join("inception/requirements-analysis")).unwrap();
    fs::write(
        record.join("inception/requirements-analysis/memory.md"),
        "# Memory\n",
    )
    .unwrap();
    let compiled = workspace.hook("rebuild-stage-graph", COMPILE_INPUT, &[]);
    assert_eq!(compiled.status.code(), Some(0), "{compiled:?}");
    assert!(record.join("runtime-graph.json").is_file());
    // 版違いの初期化印はハートビートの段で断る。
    fs::write(
        workspace.root().join("aidlc/.aidlc-runtime.state.json"),
        r#"{"version":2,"phase":"ready"}"#,
    )
    .unwrap();
    let refused = workspace.hook("rebuild-stage-graph", COMPILE_INPUT, &[]);
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
}

// ---------------------------------------------------------------------------
// log-subagent / session-end — 封筒の形と保存失敗
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_subagent_envelope_with_a_non_text_message_is_refused_and_store_failures_are_dropped() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "subagent").await;
    let refused = workspace.hook(
        "log-subagent",
        r#"{"agent_type":"aidlc-developer-agent","last_assistant_message":["not","text"]}"#,
        &[],
    );
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    assert!(!refused.stderr.is_empty());
    // 状態ファイルがディレクトリなら読めず、黙って終わる。
    let state = workspace.record().join("aidlc-state.md");
    let saved = fs::read(&state).unwrap();
    fs::remove_file(&state).unwrap();
    fs::create_dir(&state).unwrap();
    let unreadable = workspace.hook(
        "log-subagent",
        r#"{"agent_type":"aidlc-developer-agent","agent_id":"a1","last_assistant_message":"done"}"#,
        &[],
    );
    assert_eq!(unreadable.status.code(), Some(0), "{unreadable:?}");
    assert!(unreadable.stdout.is_empty() && unreadable.stderr.is_empty());
    fs::remove_dir(&state).unwrap();
    fs::write(&state, saved).unwrap();
    // ストアが塞がれていれば保存の失敗を drop に残し、ターンは止めない。
    workspace.block_store();
    let dropped = workspace.hook(
        "log-subagent",
        r#"{"agent_type":"aidlc-developer-agent","agent_id":"a1","last_assistant_message":"done"}"#,
        &[],
    );
    assert_eq!(dropped.status.code(), Some(0), "{dropped:?}");
    let drops = fs::read_dir(workspace.root().join("aidlc/spaces/default/intents"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.ends_with(".aidlc-hooks-health"))
        .or_else(|| Some(workspace.record().join(".aidlc-hooks-health")))
        .unwrap();
    assert!(
        fs::read_to_string(drops.join("log-subagent.drops")).is_ok()
            || fs::read_to_string(
                workspace
                    .record()
                    .join(".aidlc-hooks-health/log-subagent.drops")
            )
            .is_ok(),
        "drop の記録が要る"
    );
}

#[tokio::test]
async fn session_end_drops_the_audit_when_the_store_is_blocked_and_refuses_a_broken_marker() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "ending").await;
    let input = format!(r#"{{"session_id":"{SESSION}","reason":"exit"}}"#);
    let sessions = workspace.root().join("aidlc/.aidlc-sessions");
    fs::create_dir_all(&sessions).unwrap();
    fs::write(
        sessions.join(SESSION),
        format!("{}\n", intent_uuid(&workspace, &workspace.record_name())),
    )
    .unwrap();
    workspace.block_store();
    let dropped = workspace.hook("session-end", &input, &[]);
    assert_eq!(dropped.status.code(), Some(0), "{dropped:?}");
    assert!(
        workspace
            .record()
            .join(".aidlc-hooks-health/session-end.drops")
            .exists()
    );
    fs::write(
        workspace.root().join("aidlc/.aidlc-runtime.state.json"),
        r#"{"version":2,"phase":"ready"}"#,
    )
    .unwrap();
    let refused = workspace.hook("session-end", &input, &[]);
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
}

// ---------------------------------------------------------------------------
// review-freeze / reviewer-scope — 台帳やストアが無いときは開いて通す
// ---------------------------------------------------------------------------

fn reviewer_read(workspace: &Workspace, relative: &str) -> String {
    format!(
        r#"{{"tool_name":"Read","agent_type":"aidlc-architecture-reviewer-agent","tool_input":{{"file_path":"{}"}}}}"#,
        workspace.record().join(relative).display()
    )
}

#[tokio::test]
async fn reviewer_scope_opens_when_the_audit_store_is_unavailable_or_the_record_is_a_directory() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "scope").await;
    let dispatch = workspace.record().join(".aidlc-reviewer-dispatch.json");
    fs::write(
        &dispatch,
        "{\"reviewer\":\"aidlc-architecture-reviewer-agent\",\"stage\":\"functional-design\",\"unit\":\"u1-x\",\"exempt\":[]}\n",
    )
    .unwrap();
    let sibling = reviewer_read(
        &workspace,
        "construction/u2-y/functional-design/entities.md",
    );
    workspace.block_store();
    let opened = workspace.hook("reviewer-scope", &sibling, &[]);
    assert_eq!(opened.status.code(), Some(0), "{opened:?}");
    let drops = fs::read_to_string(
        workspace
            .record()
            .join(".aidlc-hooks-health/reviewer-scope.drops"),
    )
    .unwrap_or_default();
    assert!(
        drops.contains("cannot open the event store"),
        "材料が引けなければ開いて通し、原因を drop に残す: {drops}"
    );
    // 差し向け記録がディレクトリなら読めず、通す。
    fs::remove_file(&dispatch).unwrap();
    fs::create_dir(&dispatch).unwrap();
    let allowed = workspace.hook("reviewer-scope", &sibling, &[]);
    assert_eq!(allowed.status.code(), Some(0), "{allowed:?}");
}

#[tokio::test]
async fn review_freeze_opens_when_there_is_no_ledger_no_cursor_or_no_store() {
    let workspace = Workspace::new();
    let write = |path: &Path| {
        format!(
            r#"{{"tool_name":"Write","tool_input":{{"file_path":"{}"}}}}"#,
            path.display()
        )
    };
    // 記録が無い。
    let cold = workspace.hook(
        "review-freeze",
        &write(&workspace.root().join("README.md")),
        &[],
    );
    assert_eq!(cold.status.code(), Some(0), "{cold:?}");
    workspace.mint("bugfix", "freeze").await;
    let target = workspace
        .record()
        .join("inception/requirements-analysis/requirements.md");
    // カーソルが失われている。
    let cursor = workspace.record().join(".aidlc-execution");
    let saved = fs::read(&cursor).unwrap();
    fs::remove_file(&cursor).unwrap();
    let lost = workspace.hook("review-freeze", &write(&target), &[]);
    assert_eq!(lost.status.code(), Some(0), "{lost:?}");
    fs::write(&cursor, "garbage\n").unwrap();
    let broken = workspace.hook("review-freeze", &write(&target), &[]);
    assert_eq!(broken.status.code(), Some(0), "{broken:?}");
    assert!(
        fs::read_to_string(
            workspace
                .record()
                .join(".aidlc-hooks-health/review-freeze.drops")
        )
        .unwrap_or_default()
        .contains("malformed execution cursor"),
        "読めないカーソルは drop に残す"
    );
    fs::write(&cursor, saved).unwrap();
    // ストアが塞がれていれば材料が引けないので通す。
    workspace.block_store();
    let unopenable = workspace.hook("review-freeze", &write(&target), &[]);
    assert_eq!(unopenable.status.code(), Some(0), "{unopenable:?}");
    assert!(
        fs::read_to_string(
            workspace
                .record()
                .join(".aidlc-hooks-health/review-freeze.drops")
        )
        .unwrap_or_default()
        .contains("cannot open the event store")
    );
}

// ---------------------------------------------------------------------------
// aidlc-testing-posture — resolve の契約と矛盾する memory 層
// ---------------------------------------------------------------------------

fn testing_posture_case(id: &str) -> serde_json::Value {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/testing-posture.json"
    ))
    .unwrap();
    corpus
        .get("observations")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case.get("id").and_then(serde_json::Value::as_str) == Some(id))
        .unwrap()
        .clone()
}

fn write_posture_sections(workspace: &Workspace, case: &serde_json::Value) {
    let memory = workspace.root().join("aidlc/spaces/default/memory");
    for (file, body) in [
        ("team", "team.md"),
        ("project", "project.md"),
        ("org", "org.md"),
    ] {
        if let Some(section) = case.get("sections").and_then(|s| s.get(file)) {
            fs::write(
                memory.join(body),
                format!(
                    "# Rules\n\n## Testing Posture\n\n{}\n",
                    section.as_str().unwrap()
                ),
            )
            .unwrap();
        }
    }
}

#[tokio::test]
async fn testing_posture_resolve_prints_the_contract_and_reports_a_memory_conflict() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "posture").await;
    let resolved = workspace
        .invoke("aidlc-testing-posture", &["resolve"])
        .await;
    assert_eq!(resolved.code(), 0, "{resolved:?}");
    let contract: serde_json::Value = serde_json::from_str(resolved.line().unwrap()).unwrap();
    let expected = testing_posture_case("empty-bugfix-brownfield");
    assert_eq!(&contract, expected.get("contract").unwrap());

    let conflict = testing_posture_case("contradicting-project-method");
    write_posture_sections(&workspace, &conflict);
    for verb in ["resolve", "render"] {
        let refused = workspace.invoke("aidlc-testing-posture", &[verb]).await;
        assert_eq!(
            posture_error(&refused),
            conflict.get("error").unwrap().as_str().unwrap(),
            "{verb}"
        );
    }
    let fingerprint = workspace
        .invoke("aidlc-testing-posture", &["fingerprint", "--stage-level"])
        .await;
    assert_eq!(fingerprint.code(), 1, "{fingerprint:?}");
}

// ---------------------------------------------------------------------------
// deliver-stage-rules — 定義が読めなければ規則を配らず人間の作業を止めない
// ---------------------------------------------------------------------------

#[tokio::test]
async fn deliver_stage_rules_stays_silent_when_the_compiled_graph_is_unreadable() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "rules").await;
    fs::write(
        workspace.root().join(".claude/tools/data/stage-graph.json"),
        "{broken",
    )
    .unwrap();
    fs::create_dir_all(workspace.root().join(".claude/agents")).unwrap();
    fs::write(
        workspace
            .root()
            .join(".claude/agents/aidlc-developer-agent.md"),
        "# developer\n",
    )
    .unwrap();
    let output = workspace.hook(
        "deliver-stage-rules",
        r#"{"tool_name":"Task","tool_input":{"subagent_type":"aidlc-developer-agent","prompt":"Implement the unit"}}"#,
        &[],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "{output:?}"
    );
}

// ---------------------------------------------------------------------------
// write-audit-log — 相対パス・codekb・監査台帳自身・状態ファイルの無い記録
// ---------------------------------------------------------------------------

fn write_input(path: &str) -> String {
    format!(r#"{{"tool_name":"Write","tool_input":{{"file_path":"{path}"}}}}"#)
}

fn audit_text(workspace: &Workspace) -> String {
    let mut paths: Vec<PathBuf> = fs::read_dir(workspace.record().join("audit"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| fs::read_to_string(path).unwrap())
        .collect()
}

#[tokio::test]
async fn write_audit_log_resolves_relative_paths_and_codekb_but_never_audits_the_ledger_itself() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "artifacts").await;
    let before = audit_text(&workspace)
        .matches("**Event**: ARTIFACT_")
        .count();
    // 空のパスは何も書かない。
    let empty = workspace.hook("write-audit-log", &write_input(""), &[]);
    assert_eq!(empty.status.code(), Some(0), "{empty:?}");
    // 記録の外のファイルも何も書かない。
    fs::write(workspace.root().join("README.md"), "# readme\n").unwrap();
    let outside = workspace.hook("write-audit-log", &write_input("README.md"), &[]);
    assert_eq!(outside.status.code(), Some(0), "{outside:?}");
    // 監査台帳の中身は監査しない（自己参照を避ける）。
    let ledger = workspace.record().join("audit/notes.md");
    fs::write(&ledger, "# not a shard\n").unwrap();
    let ledger_write = workspace.hook(
        "write-audit-log",
        &write_input(&ledger.to_string_lossy()),
        &[],
    );
    assert_eq!(ledger_write.status.code(), Some(0), "{ledger_write:?}");
    fs::remove_file(&ledger).unwrap();
    assert_eq!(
        audit_text(&workspace)
            .matches("**Event**: ARTIFACT_")
            .count(),
        before
    );
    // 記録相対の綴りはワークスペース根から解決し、文脈は ` > ` 区切りで残る。
    let relative = format!(
        "aidlc/spaces/default/intents/{}/inception/requirements-analysis/requirements.md",
        workspace.record_name()
    );
    fs::create_dir_all(workspace.root().join(&relative).parent().unwrap()).unwrap();
    fs::write(workspace.root().join(&relative), "# req\n").unwrap();
    let recorded = workspace.hook("write-audit-log", &write_input(&relative), &[]);
    assert_eq!(recorded.status.code(), Some(0), "{recorded:?}");
    let audit = audit_text(&workspace);
    assert_eq!(audit.matches("**Event**: ARTIFACT_").count(), before + 1);
    assert!(
        audit.contains("inception > requirements-analysis > requirements.md"),
        "{audit}"
    );
    // codekb の成果物は `codekb > <repo> > <file>` の文脈で残る。
    let codekb = workspace
        .root()
        .join("aidlc/spaces/default/codekb/repo-a/scan.md");
    fs::create_dir_all(codekb.parent().unwrap()).unwrap();
    fs::write(&codekb, "# scan\n").unwrap();
    let scanned = workspace.hook(
        "write-audit-log",
        &write_input(&codekb.to_string_lossy()),
        &[],
    );
    assert_eq!(scanned.status.code(), Some(0), "{scanned:?}");
    assert!(
        audit_text(&workspace).contains("codekb > repo-a > scan.md"),
        "{}",
        audit_text(&workspace)
    );
}

#[tokio::test]
async fn write_audit_log_follows_the_cursor_when_the_record_has_no_state_file_yet() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "stateless").await;
    let record = workspace.record();
    let artifact = record.join("inception/requirements-analysis/requirements.md");
    fs::create_dir_all(artifact.parent().unwrap()).unwrap();
    fs::write(&artifact, "# req\n").unwrap();
    let state = record.join("aidlc-state.md");
    let saved = fs::read(&state).unwrap();
    fs::remove_file(&state).unwrap();
    let output = workspace.hook(
        "write-audit-log",
        &write_input(&artifact.to_string_lossy()),
        &[],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let audit: String = fs::read_dir(record.join("audit"))
        .unwrap()
        .map(|entry| fs::read_to_string(entry.unwrap().path()).unwrap())
        .collect();
    assert!(
        audit.contains("inception > requirements-analysis > requirements.md"),
        "状態ファイルが無くてもカーソルの記録へ監査を残す: {audit}"
    );
    fs::write(&state, saved).unwrap();
    // カーソルが文法外の名前を指せば投影先は決まらない（drop に残して通す）。
    fs::write(
        workspace.intents().join("active-intent"),
        "not a record name\n",
    )
    .unwrap();
    let output = workspace.hook(
        "write-audit-log",
        &write_input(&artifact.to_string_lossy()),
        &[],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
}

// ---------------------------------------------------------------------------
// 共有承認ストアの回復 — 出所のストアを失った保留操作は成功に丸めない
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_pending_publication_whose_source_store_is_gone_stops_the_next_approval_update() {
    use core_command_domain::orchestration::{
        IntentExecutionId, PlanApprovalOperationId, PlanInvalidation,
    };
    use core_command_domain::workspace::{SpaceName, StorePath};
    use core_command_interface_adapter::orchestration::PlanApprovalRuntimeRepositoryImpl;
    let workspace = Workspace::new();
    workspace.mint("bugfix", "pending").await;
    // 最初の next が共有承認ストアを初期化する。
    let first = workspace.invoke("aidlc-orchestrate", &["next"]).await;
    assert_eq!(first.code(), 0, "{first:?}");
    let runtime = StorePath::for_runtime(&workspace.root().join("aidlc"));
    assert!(runtime.as_path().is_file());
    // 別 space `other` を出所とする発行の保留を 1 件積む（その space のストアは無い）。
    let repository = PlanApprovalRuntimeRepositoryImpl::open(&runtime).unwrap();
    core_command_use_case::orchestration::PreparePlanInvalidationUseCase::new(repository)
        .execute(
            PlanInvalidation::new(
                PlanApprovalOperationId::generate(),
                SpaceName::parse("other").unwrap(),
                IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
            ),
            chrono::Utc::now(),
        )
        .await
        .unwrap();
    let second = emitted_json(&workspace.invoke("aidlc-orchestrate", &["next"]).await);
    assert_eq!(second.get("kind").unwrap(), "error");
    assert_eq!(
        second.get("message").unwrap(),
        "Pending approval publication source store is missing"
    );
}

#[tokio::test]
async fn a_malformed_stop_counter_is_observed_as_no_history_rather_than_trusted() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "counter").await;
    let counter = workspace.record().join(".aidlc-stop-hook/block-count.json");
    fs::create_dir_all(counter.parent().unwrap()).unwrap();
    for body in [
        r#"{"count":"x"}"#,
        r#"{"signature":"not a signature","count":1}"#,
        r#"{"signature":"","count":5}"#,
    ] {
        fs::write(&counter, body).unwrap();
        let output = workspace.hook("continue-workflow", r#"{"stop_hook_active":false}"#, &[]);
        assert_eq!(output.status.code(), Some(0), "{body}: {output:?}");
        let answer: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(answer.get("decision").unwrap(), "block", "{body}: {answer}");
    }
}

#[tokio::test]
async fn a_jump_observes_declared_artifacts_that_already_carry_an_extension() {
    let workspace = synthetic_hatted_workspace();
    workspace.mint("classic", "artifacts").await;
    let contract = workspace
        .record()
        .join("inception/contract-design/contract.json");
    fs::create_dir_all(contract.parent().unwrap()).unwrap();
    fs::write(&contract, "{}\n").unwrap();
    let output = workspace
        .invoke(
            "aidlc-jump",
            &[
                "execute",
                "--target",
                "contract-design",
                "--direction",
                "forward",
            ],
        )
        .await;
    let value = emitted_json(&output);
    assert_eq!(value.get("direction").unwrap(), "forward", "{value}");
}

// ---------------------------------------------------------------------------
// session-start / sync-workflow-state / heartbeat — 保存の失敗と読めない状態
// ---------------------------------------------------------------------------

#[tokio::test]
async fn session_start_surfaces_an_unreadable_state_and_drops_a_failed_audit() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "start").await;
    let input = format!(r#"{{"source":"startup","session_id":"{SESSION}"}}"#);
    // 監査の保存に失敗しても文脈は出し、原因は drop に残る。
    workspace.block_store();
    let dropped = workspace.hook("session-start", &input, &[]);
    let listing: Vec<String> = fs::read_dir(workspace.root().join("aidlc"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(dropped.status.code(), Some(0), "{dropped:?} {listing:?}");
    assert!(
        additional_context(&dropped).contains("AIDLC"),
        "{dropped:?}"
    );
    assert!(
        workspace
            .record()
            .join(".aidlc-hooks-health/session-start.drops")
            .exists()
    );
    // 状態ファイルがディレクトリなら読めず、拒否する。
    let state = workspace.record().join("aidlc-state.md");
    fs::remove_file(&state).unwrap();
    fs::create_dir(&state).unwrap();
    let unreadable = workspace.hook("session-start", &input, &[]);
    assert_eq!(unreadable.status.code(), Some(1), "{unreadable:?}");
    assert!(
        String::from_utf8_lossy(&unreadable.stderr).contains("Is a directory"),
        "{unreadable:?}"
    );
    fs::remove_dir(&state).unwrap();
    fs::write(
        &state,
        "- **Scope**: bugfix\n- **Current Stage**: reverse-engineering\n",
    )
    .unwrap();
    // コンパイル済みグラフが配列でなければ未コンパイルの stage は数えない。
    fs::write(
        workspace.root().join(".claude/tools/data/stage-graph.json"),
        "{}",
    )
    .unwrap();
    let stages = workspace
        .root()
        .join(".claude/aidlc-common/stages/inception");
    fs::create_dir_all(&stages).unwrap();
    fs::write(stages.join("mystery.md"), "# m\n").unwrap();
    // 版違いの初期化印はハートビートの段で断る。
    fs::write(
        workspace.root().join("aidlc/.aidlc-runtime.state.json"),
        r#"{"version":2,"phase":"ready"}"#,
    )
    .unwrap();
    let refused = workspace.hook("session-start", &input, &[]);
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    fs::write(
        workspace.root().join("aidlc/.aidlc-runtime.state.json"),
        r#"{"version":1,"phase":"ready"}"#,
    )
    .unwrap();
    let started = workspace.hook("session-start", &input, &[]);
    let context = additional_context(&started);
    assert!(!context.contains("mystery"), "{context}");
}

#[tokio::test]
async fn a_bound_session_that_is_stamped_elsewhere_clears_its_stamp_on_resume() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "bound").await;
    let sessions = workspace.root().join("aidlc/.aidlc-sessions");
    fs::create_dir_all(&sessions).unwrap();
    // 会話は手書きの記録（registry に無く uuid が引けない）に固定され、印は別の uuid を指す。
    let orphan = workspace.intents().join("260101-orphan-aaaaaaaa");
    fs::create_dir_all(&orphan).unwrap();
    fs::write(
        orphan.join("aidlc-state.md"),
        "- **Scope**: bugfix\n- **Current Stage**: reverse-engineering\n",
    )
    .unwrap();
    fs::write(
        sessions.join(format!("{SESSION}.binding.json")),
        r#"{"space":"default","intent":"260101-orphan-aaaaaaaa","boundAt":"2026-09-11T00:00:00Z"}"#,
    )
    .unwrap();
    fs::write(sessions.join(SESSION), "stale-uuid\n").unwrap();
    let resumed = workspace.hook(
        "session-start",
        &format!(r#"{{"source":"resume","session_id":"{SESSION}"}}"#),
        &[],
    );
    assert_eq!(resumed.status.code(), Some(0), "{resumed:?}");
    assert!(
        !sessions.join(SESSION).exists(),
        "固定と食い違う印は消える: {resumed:?}"
    );
    // 固定が無く、共有カーソルが registry に無い記録を指し、印は見つからない uuid を指す。
    fs::remove_file(sessions.join(format!("{SESSION}.binding.json"))).unwrap();
    fs::write(
        workspace.intents().join("active-intent"),
        "260101-orphan-aaaaaaaa\n",
    )
    .unwrap();
    fs::write(sessions.join(SESSION), "unknown-uuid\n").unwrap();
    let resumed = workspace.hook(
        "session-start",
        &format!(r#"{{"source":"resume","session_id":"{SESSION}"}}"#),
        &[],
    );
    assert_eq!(resumed.status.code(), Some(0), "{resumed:?}");
    assert!(
        !sessions.join(SESSION).exists(),
        "生きた intent が無ければ見つからない印は消える: {resumed:?}"
    );
}

#[tokio::test]
async fn sync_workflow_state_logs_the_failed_status_update_when_the_store_is_blocked() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "sync-fail").await;
    workspace.block_store();
    let output = workspace.hook(
        "sync-workflow-state",
        r#"{"tool_name":"TaskUpdate","tool_input":{"taskId":"1","status":"in_progress","activeForm":"Running [reverse-engineering]"}}"#,
        &[],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(
        output.stdout.is_empty(),
        "同期の失敗はターンを止めない: {output:?}"
    );
}

#[tokio::test]
async fn a_hook_heartbeat_cannot_open_a_runtime_store_that_is_a_directory() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "heartbeat").await;
    let aidlc = workspace.root().join("aidlc");
    fs::write(
        aidlc.join(".aidlc-runtime.state.json"),
        r#"{"version":1,"phase":"initializing"}"#,
    )
    .unwrap();
    fs::create_dir(aidlc.join(".aidlc-runtime.sqlite")).unwrap();
    let output = workspace.hook(
        "write-audit-log",
        r#"{"tool_name":"Write","tool_input":{"file_path":"README.md"}}"#,
        &[],
    );
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(!output.stderr.is_empty(), "{output:?}");
}

// ===========================================================================
// 2 回目（u2_cov2_app）: 失敗注入 — 塞いだストア・ディレクトリ化した保存物・壊れた投影
// ===========================================================================

impl Workspace {
    /// 起動名を `bin/<argv0>` に複製して子プロセスで打つ（環境変数で予算などを注入する）。
    fn spawn(&self, argv0: &str, args: &[&str], environment: &[(&str, &str)]) -> Output {
        let binary = self.root().join("bin").join(argv0);
        if !binary.exists() {
            fs::create_dir_all(binary.parent().unwrap()).unwrap();
            tool_link::link_tool(&binary).unwrap();
        }
        Command::new(binary)
            .args(args)
            .current_dir(self.root())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.temp.path().join("home"))
            .env("PATH", "/usr/bin:/bin")
            .env("AIDLC_DISABLE_USAGE_TRACKING", "1")
            .envs(environment.iter().copied())
            .output()
            .unwrap()
    }

    /// stdin を**ディレクトリ**にしてフックを打つ（`read_to_end` が EISDIR で失敗する）。
    fn hook_with_directory_stdin(&self, name: &str) -> Output {
        let directory = fs::File::open(self.root()).unwrap();
        Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .args(["hook", name])
            .current_dir(self.root())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.temp.path().join("home"))
            .env("PATH", "/usr/bin:/bin")
            .env("AIDLC_DISABLE_USAGE_TRACKING", "1")
            .stdin(Stdio::from(directory))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .unwrap()
    }

    /// 空間のストアを**壊れたバイト列**に差し替える（開けるが SQLite として読めない）。
    fn corrupt_store(&self) {
        fs::write(
            self.intents().join(".aidlc-store.sqlite"),
            b"this is not a sqlite database\n",
        )
        .unwrap();
    }

    /// 記録の実行カーソルを壊す。
    fn break_cursor(&self) {
        fs::write(self.record().join(".aidlc-execution"), "garbage\n").unwrap();
    }

    /// フック名ごとの drop 記録（記録付きの対象）。
    fn hook_drops(&self, hook: &str) -> String {
        fs::read_to_string(
            self.record()
                .join(".aidlc-hooks-health")
                .join(format!("{hook}.drops")),
        )
        .unwrap_or_default()
    }

    /// 監査シャードの連結。
    fn audit(&self) -> String {
        audit_text(self)
    }

    fn runtime_db(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(self.root().join("aidlc/.aidlc-runtime.sqlite")).unwrap()
    }

    fn store_db(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(self.intents().join(".aidlc-store.sqlite")).unwrap()
    }
}

/// `next` の業務拒否は終了 0 の `{"kind":"error","message":…}` である。
fn next_error(completion: &Completion) -> String {
    let value = emitted_json(completion);
    assert_eq!(value.get("kind").unwrap(), "error", "{value}");
    value.get("message").unwrap().as_str().unwrap().to_string()
}

// ---------------------------------------------------------------------------
// write-audit-log — 台帳・成果物・ストアの各前提が欠けたら drop に残して通す
// ---------------------------------------------------------------------------

#[tokio::test]
async fn write_audit_log_drops_each_broken_ledger_precondition_by_cause() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "ledger").await;
    let record = workspace.record();
    let artifact = record.join("inception/requirements-analysis/requirements.md");
    fs::create_dir_all(artifact.parent().unwrap()).unwrap();
    fs::write(&artifact, "# req\n").unwrap();
    let input = write_input(&artifact.to_string_lossy());
    let audit_dir = record.join("audit");
    let shards: Vec<(PathBuf, Vec<u8>)> = fs::read_dir(&audit_dir)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let bytes = fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect();
    assert!(!shards.is_empty());

    // 監査ディレクトリがファイルなら列挙できない。
    fs::remove_dir_all(&audit_dir).unwrap();
    fs::write(&audit_dir, "not a directory\n").unwrap();
    let blocked = workspace.hook("write-audit-log", &input, &[]);
    assert_eq!(blocked.status.code(), Some(0), "{blocked:?}");
    assert!(blocked.stdout.is_empty(), "{blocked:?}");
    let drops = workspace.hook_drops("write-audit-log");
    assert!(
        drops.contains("Not a directory"),
        "台帳が列挙できない原因を drop に残す: {drops}"
    );

    // 監査ディレクトリはあるがシャード（`.md`）が無い。
    fs::remove_file(&audit_dir).unwrap();
    fs::create_dir(&audit_dir).unwrap();
    let empty = workspace.hook("write-audit-log", &input, &[]);
    assert_eq!(empty.status.code(), Some(0), "{empty:?}");
    assert!(
        workspace
            .hook_drops("write-audit-log")
            .contains("active audit shard is missing"),
        "{}",
        workspace.hook_drops("write-audit-log")
    );

    // シャードを戻す。存在しない成果物の `Write` は metadata が取れない。
    for (path, bytes) in &shards {
        fs::write(path, bytes).unwrap();
    }
    // 相対パスは（正規化済みの）project dir から解決される。
    let missing = format!(
        "aidlc/spaces/default/intents/{}/inception/requirements-analysis/never-written.md",
        workspace.record_name()
    );
    let output = workspace.hook("write-audit-log", &write_input(&missing), &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(
        workspace
            .hook_drops("write-audit-log")
            .contains("No such file or directory"),
        "{}",
        workspace.hook_drops("write-audit-log")
    );

    // 空間ストアへの追記が拒まれても、フックは通し、原因を drop に残す。
    let before = workspace.audit();
    let db = workspace.store_db();
    db.execute_batch("CREATE TRIGGER fail_artifact BEFORE INSERT ON journal WHEN NEW.manifest='artifact-audit-event/1' BEGIN SELECT RAISE(ABORT,'injected artifact audit failure'); END;").unwrap();
    let refused = workspace.hook("write-audit-log", &input, &[]);
    assert_eq!(refused.status.code(), Some(0), "{refused:?}");
    assert!(
        workspace.hook_drops("write-audit-log").contains("conflict"),
        "{}",
        workspace.hook_drops("write-audit-log")
    );
    assert_eq!(workspace.audit(), before, "拒まれた追記は台帳へ描かれない");
    db.execute_batch("DROP TRIGGER fail_artifact").unwrap();
    drop(db);

    // 空間ストアが SQLite として読めなければ開けない。
    workspace.corrupt_store();
    let corrupt = workspace.hook("write-audit-log", &input, &[]);
    assert_eq!(corrupt.status.code(), Some(0), "{corrupt:?}");
    let drops = workspace.hook_drops("write-audit-log");
    assert!(
        drops.lines().count() >= 4,
        "壊れたストアも drop として数える: {drops}"
    );
}

#[tokio::test]
async fn write_audit_log_without_a_state_file_needs_the_active_intent_cursor() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "cursorless").await;
    let record = workspace.record();
    let artifact = record.join("inception/requirements-analysis/requirements.md");
    fs::create_dir_all(artifact.parent().unwrap()).unwrap();
    fs::write(&artifact, "# req\n").unwrap();
    let input = write_input(&artifact.to_string_lossy());
    let cursor = workspace.intents().join("active-intent");
    let saved = fs::read(&cursor).unwrap();
    fs::remove_file(record.join("aidlc-state.md")).unwrap();
    // カーソルが無いと投影先の記録は決まらない（drop は space 段に残る）。
    fs::remove_file(&cursor).unwrap();
    let output = workspace.hook("write-audit-log", &input, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let space_drops = fs::read_to_string(
        workspace
            .intents()
            .join(".aidlc-hooks-health/write-audit-log.drops"),
    )
    .unwrap_or_default();
    assert!(
        space_drops.contains("No such file or directory"),
        "{space_drops}"
    );
    // カーソルがディレクトリでも同じく読めない。
    fs::create_dir(&cursor).unwrap();
    let output = workspace.hook("write-audit-log", &input, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    fs::remove_dir(&cursor).unwrap();
    // カーソルが文法外の名前なら投影先は決まらない。
    fs::write(&cursor, "not a record name\n").unwrap();
    let output = workspace.hook("write-audit-log", &input, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(
        space_drops_now(&workspace).contains("active artifact record is invalid"),
        "{}",
        space_drops_now(&workspace)
    );
    fs::write(&cursor, saved).unwrap();
}

fn space_drops_now(workspace: &Workspace) -> String {
    fs::read_to_string(
        workspace
            .intents()
            .join(".aidlc-hooks-health/write-audit-log.drops"),
    )
    .unwrap_or_default()
}

#[test]
fn a_hook_whose_stdin_cannot_be_read_stays_silent() {
    let workspace = Workspace::new();
    let output = workspace.hook_with_directory_stdin("write-audit-log");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "{output:?}"
    );
}

// ---------------------------------------------------------------------------
// record-human-turn — 共有承認ストアが開けなくても人間ターンの監査は残す
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_human_turn_is_still_audited_when_the_shared_approval_lock_cannot_be_opened() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "prompt").await;
    let first = workspace.invoke("aidlc-orchestrate", &["next"]).await;
    assert_eq!(first.code(), 0, "{first:?}");
    let lock = workspace.root().join("aidlc/.aidlc-runtime.lock");
    let _ = fs::remove_file(&lock);
    fs::create_dir(&lock).unwrap();
    let before = workspace.audit().matches("**Session**: human-one").count();
    let output = workspace.hook(
        "record-human-turn",
        r#"{"session_id":"human-one","prompt":"Approve"}"#,
        &[],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        workspace.audit().matches("**Session**: human-one").count(),
        before + 1,
        "共有側が開けなくても人間の発言は空間ストアへ記録する: {}",
        workspace.audit()
    );
    fs::remove_dir(&lock).unwrap();

    // カーソルが壊れた記録では何も記録しない（フックは沈黙）。
    workspace.break_cursor();
    let before = workspace.audit();
    let output = workspace.hook(
        "record-human-turn",
        r#"{"session_id":"human-two","prompt":"Approve"}"#,
        &[],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(workspace.audit(), before);
}

#[tokio::test]
async fn a_refused_prompt_observation_never_reaches_the_ledger() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "refused").await;
    // 共有承認ストアがまだ無いので、人間ターンは空間ストアへ直接記録する。
    let db = workspace.store_db();
    db.execute_batch("CREATE TRIGGER fail_prompt BEFORE INSERT ON journal WHEN CAST(NEW.payload AS TEXT) LIKE '%PromptObserved%' BEGIN SELECT RAISE(ABORT,'injected prompt failure'); END;").unwrap();
    let before = workspace.audit();
    let output = workspace.hook(
        "record-human-turn",
        r#"{"session_id":"human-three","prompt":"Approve"}"#,
        &[],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(workspace.audit(), before);
    db.execute_batch("DROP TRIGGER fail_prompt").unwrap();
    // 壊れたストアでも同じく通す（記録は残らない）。
    workspace.corrupt_store();
    let output = workspace.hook(
        "record-human-turn",
        r#"{"session_id":"human-four","prompt":"Approve"}"#,
        &[],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
}

// ---------------------------------------------------------------------------
// aidlc-log decision — rationale の同伴と保存拒否
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_decision_carries_its_rationale_and_a_refused_write_names_the_cause() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "rationale").await;
    let recorded = workspace
        .invoke(
            "aidlc-log",
            &[
                "decision",
                "--stage",
                "reverse-engineering",
                "--decision",
                "Pick the scan depth",
                "--options",
                "Shallow,Deep",
                "--rationale",
                "Deep scans cost more.",
            ],
        )
        .await;
    let value = emitted_json(&recorded);
    assert_eq!(
        value.get("emitted").unwrap(),
        "DECISION_RECORDED",
        "{value}"
    );
    assert!(
        workspace.audit().contains("Deep scans cost more."),
        "{}",
        workspace.audit()
    );
    let db = workspace.store_db();
    db.execute_batch("CREATE TRIGGER fail_decision BEFORE INSERT ON journal BEGIN SELECT RAISE(ABORT,'injected decision failure'); END;").unwrap();
    let refused = workspace
        .invoke(
            "aidlc-log",
            &[
                "decision",
                "--stage",
                "reverse-engineering",
                "--decision",
                "Pick again",
            ],
        )
        .await;
    let message = refused_error(&refused);
    assert!(
        message.starts_with("Audit emission failed: repository: "),
        "拒まれた追記は保存側の原因で断る: {message}"
    );
}

// ---------------------------------------------------------------------------
// aidlc-jump — 未知の対象・壊れたストア・ソース予算
// ---------------------------------------------------------------------------

#[tokio::test]
async fn jump_execute_refuses_an_unknown_target_and_a_corrupt_store() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "jump").await;
    let unknown = workspace
        .invoke(
            "aidlc-jump",
            &[
                "execute",
                "--target",
                "Not A Slug",
                "--direction",
                "forward",
            ],
        )
        .await;
    assert_eq!(refused_error(&unknown), "Unknown stage: Not A Slug");
    let unknown_stage = workspace
        .invoke("aidlc-jump", &["resolve", "--stage", "no-such-stage"])
        .await;
    assert_eq!(
        refused_error(&unknown_stage),
        "Unknown stage: no-such-stage"
    );
    workspace.corrupt_store();
    let corrupt = workspace
        .invoke(
            "aidlc-jump",
            &[
                "execute",
                "--target",
                "intent-capture",
                "--direction",
                "backward",
            ],
        )
        .await;
    assert_eq!(corrupt.code(), 1, "{corrupt:?}");
    assert_eq!(
        refused_error(&corrupt),
        format!(
            "io: InvalidData at {}",
            workspace.intents().join(".aidlc-store.sqlite").display()
        )
    );
}

#[tokio::test]
async fn source_baseline_budgets_leave_a_git_workspace_unbindable_instead_of_failing() {
    let workspace = Workspace::new();
    // `.git` が在る根では採取失敗を空の一覧に丸めない（`unbindable` になる）。
    fs::create_dir(workspace.root().join(".git")).unwrap();
    fs::create_dir_all(workspace.root().join("src/nested")).unwrap();
    fs::write(workspace.root().join("src/nested/a.rs"), "// a\n").unwrap();
    std::os::unix::fs::symlink("lib.rs", workspace.root().join("src/one.rs")).unwrap();
    std::os::unix::fs::symlink("lib.rs", workspace.root().join("src/two.rs")).unwrap();
    let minted = workspace.spawn(
        "aidlc-utility",
        &[
            "intent-create",
            "--scope",
            "bugfix",
            "--label",
            "budget",
            "--arguments",
            "Fix one small defect",
        ],
        &[("AIDLC_TEST_SOURCE_MAX_ENTRIES", "1")],
    );
    assert_eq!(minted.status.code(), Some(0), "{minted:?}");
    let unbindable = |audit: &str| audit.matches("**Source Baseline**: unbindable").count();
    assert_eq!(unbindable(&workspace.audit()), 1, "{}", workspace.audit());
    let snapshots = workspace
        .record()
        .join(".aidlc-source-review/code-generation");
    assert!(!snapshots.exists(), "予算超過の基準は snapshot を書かない");
    for (index, variable) in [
        "AIDLC_TEST_SOURCE_MAX_DIRECTORIES",
        "AIDLC_TEST_SOURCE_MAX_ENTRIES",
        "AIDLC_TEST_SOURCE_MAX_SYMLINKS",
    ]
    .into_iter()
    .enumerate()
    {
        let jumped = workspace.spawn(
            "aidlc-jump",
            &[
                "execute",
                "--target",
                "reverse-engineering",
                "--direction",
                "redo",
            ],
            &[(variable, "1")],
        );
        assert_eq!(jumped.status.code(), Some(0), "{variable}: {jumped:?}");
        // 1 回の jump は監査に基準を 2 度描く（境界と結果）。
        assert_eq!(
            unbindable(&workspace.audit()),
            1 + 2 * (index + 1),
            "{variable}"
        );
        assert!(!snapshots.exists(), "{variable}");
    }
    // 予算内なら同じ redo が基準の snapshot を残す。
    let jumped = workspace.spawn(
        "aidlc-jump",
        &[
            "execute",
            "--target",
            "reverse-engineering",
            "--direction",
            "redo",
        ],
        &[],
    );
    assert_eq!(jumped.status.code(), Some(0), "{jumped:?}");
    assert_eq!(
        unbindable(&workspace.audit()),
        7,
        "予算内の基準は unbindable にならない"
    );
    assert!(
        snapshots.is_dir() && fs::read_dir(&snapshots).unwrap().count() == 1,
        "{}",
        String::from_utf8_lossy(&jumped.stdout)
    );
}

// ---------------------------------------------------------------------------
// aidlc-testing-posture — 壊れたカーソル・壊れたストア・不正な unit
// ---------------------------------------------------------------------------

#[tokio::test]
async fn testing_posture_refuses_a_broken_cursor_a_corrupt_store_and_a_malformed_unit() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "posture").await;
    let unit = workspace
        .invoke(
            "aidlc-testing-posture",
            &["fingerprint", "--unit", "Bad Unit!"],
        )
        .await;
    assert!(
        posture_error(&unit).starts_with("Invalid Unit name \"Bad Unit!\""),
        "{:?}",
        posture_error(&unit)
    );
    let begin_unit = workspace
        .invoke("aidlc-testing-posture", &["begin", "--unit", "Bad Unit!"])
        .await;
    assert_eq!(posture_error(&begin_unit), posture_error(&unit));
    workspace.break_cursor();
    for args in [
        vec!["render"],
        vec!["fingerprint", "--stage-level"],
        vec!["begin", "--stage-level"],
    ] {
        let broken = workspace.invoke("aidlc-testing-posture", &args).await;
        assert!(
            posture_error(&broken).contains("garbage") || posture_error(&broken).contains("cursor"),
            "{args:?}: {}",
            posture_error(&broken)
        );
    }
    workspace.corrupt_store();
    let corrupt = workspace.invoke("aidlc-testing-posture", &["render"]).await;
    assert!(
        posture_error(&corrupt).starts_with("io: InvalidData at "),
        "{}",
        posture_error(&corrupt)
    );
}

// ---------------------------------------------------------------------------
// aidlc-learnings — 選択ファイルの形・ロック・memory 層の読取失敗
// ---------------------------------------------------------------------------

#[tokio::test]
async fn learnings_persist_refuses_malformed_selections_and_a_blocked_lock() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "shape").await;
    let stage = workspace.current_stage();
    let selections = workspace.root().join("selections.json");
    let arg = selections.to_string_lossy().into_owned();
    let persist = || async {
        workspace
            .invoke("aidlc-learnings", &["persist", "--selections-json", &arg])
            .await
    };
    // 読めない（ディレクトリ）。
    fs::create_dir(&selections).unwrap();
    let unreadable = refused_plain(&persist().await);
    assert!(
        unreadable.starts_with("selections-json is malformed: ")
            && unreadable.contains("Is a directory"),
        "{unreadable}"
    );
    fs::remove_dir(&selections).unwrap();
    // 欄の欠落と文法外の stage。
    for body in [
        r#"{"space":"default","intent":"x","selections":[]}"#,
        r#"{"stage_slug":"Not A Slug","space":"default","intent":"x","selections":[]}"#,
    ] {
        fs::write(&selections, body).unwrap();
        assert_eq!(
            refused_plain(&persist().await),
            aidlc::wording::LEARNINGS_SELECTIONS_SHAPE,
            "{body}"
        );
    }
    let record = workspace.record_name();
    fs::write(
        &selections,
        format!(r#"{{"stage_slug":"{stage}","space":"default","intent":"{record}"}}"#),
    )
    .unwrap();
    assert_eq!(
        refused_plain(&persist().await),
        aidlc::wording::LEARNINGS_SELECTIONS_SHAPE
    );
    // ロックファイルがディレクトリなら直列化できない。
    fs::write(
        &selections,
        format!(
            r#"{{"stage_slug":"{stage}","space":"default","intent":"{record}","selections":[]}}"#
        ),
    )
    .unwrap();
    let lock = workspace.root().join("aidlc/.aidlc-learnings.lock");
    fs::create_dir(&lock).unwrap();
    let locked = refused_plain(&persist().await);
    assert!(
        locked.starts_with(&aidlc::wording::learnings_persist_failed("")),
        "{locked}"
    );
    fs::remove_dir(&lock).unwrap();
    // memory 層のファイルが在るのに読めなければ実測できない（存在検査は通る）。
    use std::os::unix::fs::PermissionsExt as _;
    let team = workspace.root().join("aidlc/spaces/default/memory/team.md");
    fs::set_permissions(&team, fs::Permissions::from_mode(0o000)).unwrap();
    let unreadable = refused_plain(&persist().await);
    fs::set_permissions(&team, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(
        unreadable.contains("PermissionDenied"),
        "投影の復元が先に読めない事実を報告する: {unreadable}"
    );
}

#[tokio::test]
async fn learnings_surface_refuses_an_unreadable_state_and_a_graph_without_stages() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "graphless").await;
    let stage = workspace.current_stage();
    let record = workspace.record();
    let graph = record.join("runtime-graph.json");
    fs::write(&graph, "{}").unwrap();
    let no_stages = workspace
        .invoke("aidlc-learnings", &["surface", "--slug", &stage])
        .await;
    assert_eq!(
        refused_plain(&no_stages),
        aidlc::wording::LEARNINGS_RUNTIME_GRAPH_NO_STAGES
    );
    fs::remove_file(&graph).unwrap();
    fs::create_dir(&graph).unwrap();
    let unreadable = workspace
        .invoke("aidlc-learnings", &["surface", "--slug", &stage])
        .await;
    assert!(
        refused_plain(&unreadable).contains("Is a directory"),
        "{:?}",
        refused_plain(&unreadable)
    );
    fs::remove_dir(&graph).unwrap();
    // 状態ファイルがディレクトリなら記録は選ばれるが読めない。
    let state = record.join("aidlc-state.md");
    fs::remove_file(&state).unwrap();
    fs::create_dir(&state).unwrap();
    let broken = workspace
        .invoke("aidlc-learnings", &["surface", "--slug", &stage])
        .await;
    assert_eq!(broken.code(), 1, "{broken:?}");
    assert!(
        broken
            .diagnostic()
            .unwrap()
            .contains("publication conflict"),
        "投影の復元がディレクトリと衝突する: {broken:?}"
    );
}

// ---------------------------------------------------------------------------
// aidlc-log answer — summary-confirmation の質問ファイル指定
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_summary_confirmation_needs_a_readable_questions_file() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "summary").await;
    let without = workspace
        .invoke(
            "aidlc-log",
            &[
                "answer",
                "--stage",
                "reverse-engineering",
                "--checkpoint",
                "summary-confirmation",
                "--details",
                "Looks correct",
            ],
        )
        .await;
    assert_eq!(
        refused_error(&without),
        "Summary confirmation requires --questions-file <path> so the receipt can bind to the reviewed answers."
    );
    let directory = workspace
        .record()
        .join("inception/reverse-engineering/reverse-engineering-questions.md");
    fs::create_dir_all(&directory).unwrap();
    let relative = directory
        .strip_prefix(workspace.root())
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let unreadable = workspace
        .invoke(
            "aidlc-log",
            &[
                "answer",
                "--stage",
                "reverse-engineering",
                "--checkpoint",
                "summary-confirmation",
                "--details",
                "Looks correct",
                "--questions-file",
                &relative,
            ],
        )
        .await;
    assert!(
        refused_error(&unreadable).contains("Is a directory"),
        "{:?}",
        refused_error(&unreadable)
    );
}

// ---------------------------------------------------------------------------
// 共有承認ストアの回復 — 壊れた保留行はその欠陥の名前で止まる
// ---------------------------------------------------------------------------

/// 別 space `other` を出所とする発行の保留を 1 件積む（その space のストアは無い）。
async fn prepare_pending_publication_from_another_space(workspace: &Workspace) {
    use core_command_domain::orchestration::{
        IntentExecutionId, PlanApprovalOperationId, PlanInvalidation,
    };
    use core_command_domain::workspace::{SpaceName, StorePath};
    use core_command_interface_adapter::orchestration::PlanApprovalRuntimeRepositoryImpl;
    let runtime = StorePath::for_runtime(&workspace.root().join("aidlc"));
    assert!(runtime.as_path().is_file());
    let repository = PlanApprovalRuntimeRepositoryImpl::open(&runtime).unwrap();
    core_command_use_case::orchestration::PreparePlanInvalidationUseCase::new(repository)
        .execute(
            PlanInvalidation::new(
                PlanApprovalOperationId::generate(),
                SpaceName::parse("other").unwrap(),
                IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
            ),
            chrono::Utc::now(),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn a_broken_pending_approval_row_is_refused_by_its_exact_defect() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "rows").await;
    let first = workspace.invoke("aidlc-orchestrate", &["next"]).await;
    assert_eq!(first.code(), 0, "{first:?}");
    prepare_pending_publication_from_another_space(&workspace).await;
    let db = workspace.runtime_db();
    let uuid_error = "not a canonical UUIDv7 (expected lowercase 8-4-4-4-12)";
    for (update, expected) in [
        (
            "SET kind='response'",
            "Pending human response source store is missing",
        ),
        (
            "SET kind='answer'",
            "Pending plan answer source store is missing",
        ),
        (
            "SET kind='ritual'",
            "Unknown pending approval operation kind: ritual",
        ),
        (
            "SET space=NULL",
            "Pending approval operation has no source space",
        ),
        (
            "SET space='not a space!'",
            "Pending approval operation has invalid source space",
        ),
        (
            "SET space='other', execution_id=NULL",
            "Pending approval operation has no source execution",
        ),
        ("SET execution_id='nope'", uuid_error),
        ("SET operation_id='x'", uuid_error),
    ] {
        // 投影は行を描き直すので、描かれた直後に壊す引き金で欠陥を注入する。
        db.execute_batch(&format!(
            "CREATE TRIGGER break_row AFTER INSERT ON read_plan_operation WHEN NEW.status='prepared' BEGIN UPDATE read_plan_operation {update} WHERE operation_id=NEW.operation_id; END; UPDATE read_plan_operation {update} WHERE status='prepared';"
        ))
        .unwrap();
        let refused = next_error(&workspace.invoke("aidlc-orchestrate", &["next"]).await);
        db.execute_batch("DROP TRIGGER break_row").unwrap();
        assert_eq!(refused, expected, "{update}");
    }
}

#[tokio::test]
async fn the_shared_approval_lock_refuses_a_directory_and_a_held_lock() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "lock").await;
    let first = workspace.invoke("aidlc-orchestrate", &["next"]).await;
    assert_eq!(first.code(), 0, "{first:?}");
    let lock = workspace.root().join("aidlc/.aidlc-runtime.lock");
    let _ = fs::remove_file(&lock);
    fs::create_dir(&lock).unwrap();
    let directory = next_error(&workspace.invoke("aidlc-orchestrate", &["next"]).await);
    assert_eq!(directory, "Is a directory (os error 21)");
    fs::remove_dir(&lock).unwrap();
    // 別のハンドルが握っている間は 5 秒待って諦める。
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock)
        .unwrap();
    let held =
        core_infrastructure::ExclusiveFileLock::acquire(file, std::time::Duration::from_secs(1))
            .unwrap();
    let started = std::time::Instant::now();
    let timed_out = next_error(&workspace.invoke("aidlc-orchestrate", &["next"]).await);
    assert_eq!(timed_out, "exclusive file lock timed out");
    assert!(started.elapsed() >= std::time::Duration::from_secs(5));
    drop(held);
}

// ---------------------------------------------------------------------------
// continue-workflow (Stop) — 排他・状態・エンジンの各失敗は drop に残して通す
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_stop_allows_the_turn_when_the_runtime_lock_is_unavailable() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "stoplock").await;
    let first = workspace.invoke("aidlc-orchestrate", &["next"]).await;
    assert_eq!(first.code(), 0, "{first:?}");
    let lock = workspace.root().join("aidlc/.aidlc-runtime.lock");
    let _ = fs::remove_file(&lock);
    fs::create_dir(&lock).unwrap();
    // 共有 resume 待ちの証拠を読む前に排他が取れない。
    let output = workspace.hook("continue-workflow", r#"{"stop_hook_active":false}"#, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(
        drops(&workspace).contains(
            "active-directive evidence unavailable while reading shared resume wait: Is a directory (os error 21); allowing stop"
        ),
        "{}",
        drops(&workspace)
    );
    // 指示の証拠が無くても、停止要求の保存で同じ排他が要る。
    fs::remove_file(workspace.record().join(".aidlc-active-directive.json")).unwrap();
    let output = workspace.hook("continue-workflow", r#"{"stop_hook_active":false}"#, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert_eq!(
        drops(&workspace)
            .matches("Is a directory (os error 21)")
            .count(),
        2,
        "{}",
        drops(&workspace)
    );
    fs::remove_dir(&lock).unwrap();
    // 別プロセスが握っている間は 1 秒で諦める。
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock)
        .unwrap();
    let held =
        core_infrastructure::ExclusiveFileLock::acquire(file, std::time::Duration::from_secs(1))
            .unwrap();
    let output = workspace.hook("continue-workflow", r#"{"stop_hook_active":false}"#, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    drop(held);
    assert!(
        drops(&workspace).contains("exclusive file lock timed out"),
        "{}",
        drops(&workspace)
    );
}

#[tokio::test]
async fn a_stop_allows_the_turn_when_the_state_or_the_engine_cannot_be_read() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "stopstate").await;
    let first = workspace.invoke("aidlc-orchestrate", &["next"]).await;
    assert_eq!(first.code(), 0, "{first:?}");
    // 空間ストアが読めなければ停止要求を保存できず、停止を許す。
    workspace.corrupt_store();
    let output = workspace.hook("continue-workflow", r#"{"stop_hook_active":false}"#, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(
        drops(&workspace).contains(".aidlc-store.sqlite: invalid data"),
        "{}",
        drops(&workspace)
    );
    // 状態ファイルがディレクトリなら読めない（記録は選ばれる）。
    let state = workspace.record().join("aidlc-state.md");
    fs::remove_file(&state).unwrap();
    fs::create_dir(&state).unwrap();
    let output = workspace.hook("continue-workflow", r#"{"stop_hook_active":false}"#, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(
        drops(&workspace).contains("Is a directory (os error 21)"),
        "{}",
        drops(&workspace)
    );
}

#[tokio::test]
async fn a_stop_allows_the_turn_when_the_engine_refuses_to_answer() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "stopengine").await;
    // 文法外の space では子プロセスの `next` が終了 1 で断るので、停止を許す
    // （drop 先の space 名も文法外なので記録は残らない）。
    workspace.move_into_an_invalid_space();
    let output = workspace.hook("continue-workflow", r#"{"stop_hook_active":false}"#, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "{output:?}"
    );
}

#[tokio::test]
async fn a_stop_stays_silent_when_the_runtime_store_itself_is_unreadable() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "stopruntime").await;
    let first = workspace.invoke("aidlc-orchestrate", &["next"]).await;
    assert_eq!(first.code(), 0, "{first:?}");
    fs::write(
        workspace.root().join("aidlc/.aidlc-runtime.sqlite"),
        b"this is not a sqlite database\n",
    )
    .unwrap();
    let state = workspace.state();
    let output = workspace.hook("continue-workflow", r#"{"stop_hook_active":false}"#, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert_eq!(workspace.state(), state, "停止制御はワークフローを進めない");
}

// ---------------------------------------------------------------------------
// validate-state (PreCompact) — breadcrumb・監査・失効の各失敗
// ---------------------------------------------------------------------------

#[tokio::test]
async fn validate_state_refuses_an_unwritable_breadcrumb_and_drops_a_refused_audit() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "compact").await;
    let first = workspace.invoke("aidlc-orchestrate", &["next"]).await;
    assert_eq!(first.code(), 0, "{first:?}");
    let input = format!(r#"{{"session_id":"{SESSION}"}}"#);
    let breadcrumb = workspace.record().join(".aidlc-recovery.md");
    let _ = fs::remove_file(&breadcrumb);
    fs::create_dir(&breadcrumb).unwrap();
    let refused = workspace.hook("validate-state", &input, &[]);
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("Is a directory"),
        "{refused:?}"
    );
    fs::remove_dir(&breadcrumb).unwrap();
    // 監査の追記が拒まれても breadcrumb は書き、原因を drop に残す。
    let db = workspace.store_db();
    db.execute_batch("CREATE TRIGGER fail_session_audit BEFORE INSERT ON journal WHEN NEW.manifest='session-audit-event/1' BEGIN SELECT RAISE(ABORT,'injected session audit failure'); END;").unwrap();
    let dropped = workspace.hook("validate-state", &input, &[]);
    assert_eq!(dropped.status.code(), Some(0), "{dropped:?}");
    assert!(breadcrumb.is_file());
    assert!(
        workspace.hook_drops("validate-state").contains("conflict"),
        "{}",
        workspace.hook_drops("validate-state")
    );
    db.execute_batch("DROP TRIGGER fail_session_audit").unwrap();
    drop(db);
    // 空間ストアが読めなければ失効も監査もできないが、フックは通す。
    workspace.corrupt_store();
    let corrupt = workspace.hook("validate-state", &input, &[]);
    assert_eq!(corrupt.status.code(), Some(0), "{corrupt:?}");
    assert!(
        workspace
            .hook_drops("validate-state")
            .contains("InvalidData"),
        "{}",
        workspace.hook_drops("validate-state")
    );
}

#[tokio::test]
async fn validate_state_records_nothing_without_a_clone_identity() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "cloneless").await;
    fs::remove_file(workspace.root().join("aidlc/.aidlc-clone-id")).unwrap();
    let before = workspace.audit();
    let output = workspace.hook("validate-state", r#"{"session_id":"none"}"#, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        workspace.audit(),
        before,
        "clone id 無しでは当該シャードを特定できない"
    );
}

// ---------------------------------------------------------------------------
// 投影の前提が壊れたときの書込動詞 — 書けても描けなければ断る
// ---------------------------------------------------------------------------

#[tokio::test]
async fn every_write_verb_refuses_when_the_clone_identity_cannot_be_minted() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "cloneid").await;
    let first = workspace.invoke("aidlc-orchestrate", &["next"]).await;
    assert_eq!(first.code(), 0, "{first:?}");
    let clone = workspace.root().join("aidlc/.aidlc-clone-id");
    fs::remove_file(&clone).unwrap();
    fs::create_dir(&clone).unwrap();
    let expected_prefix = aidlc::wording::orchestrate_failure("clone id: ");
    let decision = workspace
        .invoke(
            "aidlc-log",
            &[
                "decision",
                "--stage",
                "reverse-engineering",
                "--decision",
                "Pick",
            ],
        )
        .await;
    assert!(
        refused_error(&decision).starts_with(&expected_prefix),
        "{decision:?}"
    );
    let autonomy = workspace
        .invoke("aidlc-bolt", &["set-autonomy", "--mode", "autonomous"])
        .await;
    assert!(
        refused_plain(&autonomy).starts_with(&expected_prefix),
        "{autonomy:?}"
    );
    requirements_documents(&workspace.record(), false);
    let review = workspace
        .invoke(
            "aidlc-log",
            &[
                "review",
                "--stage",
                "requirements-analysis",
                "--reviewer",
                "aidlc-product-lead-agent",
                "--iteration",
                "1",
            ],
        )
        .await;
    assert!(
        refused_error(&review).starts_with(&expected_prefix),
        "{review:?}"
    );
    let selections = workspace.root().join("selections.json");
    fs::write(
        &selections,
        format!(
            r#"{{"stage_slug":"{}","space":"default","intent":"{}","selections":[]}}"#,
            workspace.current_stage(),
            workspace.record_name()
        ),
    )
    .unwrap();
    let persisted = workspace
        .invoke(
            "aidlc-learnings",
            &[
                "persist",
                "--selections-json",
                &selections.to_string_lossy(),
            ],
        )
        .await;
    assert!(
        refused_plain(&persisted).starts_with(&expected_prefix),
        "{persisted:?}"
    );
    let reported = workspace
        .invoke(
            "aidlc-orchestrate",
            &["report", "--result", "awaiting-approval"],
        )
        .await;
    assert_eq!(reported.code(), 1, "{reported:?}");
    assert!(
        reported.diagnostic().unwrap().starts_with(&expected_prefix),
        "{reported:?}"
    );
}

#[tokio::test]
async fn a_report_needs_a_readable_stage_graph_for_its_source_baseline() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "graph").await;
    let graph = workspace.root().join(".claude/tools/data/stage-graph.json");
    let saved = fs::read(&graph).unwrap();
    // `skipped` は基準の採取（`for_report`）を要する verdict である。
    let report = || async {
        next_error(
            &workspace
                .invoke(
                    "aidlc-orchestrate",
                    &[
                        "report",
                        "--result",
                        "skipped",
                        "--stage",
                        "reverse-engineering",
                    ],
                )
                .await,
        )
    };
    fs::remove_file(&graph).unwrap();
    assert!(
        report().await.contains("No such file or directory"),
        "{}",
        report().await
    );
    fs::write(&graph, b"{}").unwrap();
    assert_eq!(
        report().await,
        "invalid type: map, expected a sequence at line 1 column 0"
    );
    fs::write(
        &graph,
        br#"[{"workspace_requires":true,"slug":"Not A Slug"}]"#,
    )
    .unwrap();
    let invalid = report().await;
    assert!(
        !invalid.is_empty() && !invalid.contains("No such file"),
        "{invalid}"
    );
    fs::write(&graph, br#"[{"workspace_requires":true}]"#).unwrap();
    assert_eq!(report().await, "workspace stage has no slug");
    fs::write(&graph, saved).unwrap();
}

#[tokio::test]
async fn a_corrupt_space_store_stops_report_and_autonomy_switches_by_name() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "corrupt").await;
    workspace.corrupt_store();
    let reported = workspace
        .invoke(
            "aidlc-orchestrate",
            &["report", "--result", "awaiting-approval"],
        )
        .await;
    assert_eq!(reported.code(), 1, "{reported:?}");
    assert!(
        reported
            .diagnostic()
            .unwrap()
            .contains("journal: io: InvalidData"),
        "{reported:?}"
    );
    let autonomy = workspace
        .invoke("aidlc-bolt", &["set-autonomy", "--mode", "autonomous"])
        .await;
    assert!(
        refused_plain(&autonomy).contains("journal: io: InvalidData"),
        "{autonomy:?}"
    );
}

#[tokio::test]
async fn next_and_mint_refuse_when_the_space_cannot_hold_an_intents_directory() {
    let workspace = Workspace::new();
    let space = workspace.root().join("aidlc/spaces/default");
    fs::remove_dir_all(&space).unwrap();
    fs::write(&space, "not a directory\n").unwrap();
    let next = next_error(&workspace.invoke("aidlc-orchestrate", &["next"]).await);
    assert_eq!(
        next,
        "aidlc-orchestrate: cannot create the record directory: Not a directory (os error 20)"
    );
    let minted = workspace
        .invoke(
            "aidlc-utility",
            &[
                "intent-create",
                "--scope",
                "bugfix",
                "--label",
                "blocked",
                "--arguments",
                "Fix one small defect",
            ],
        )
        .await;
    assert_eq!(minted.code(), 1, "{minted:?}");
    assert!(
        minted
            .diagnostic()
            .unwrap()
            .contains("cannot create the record directory"),
        "{minted:?}"
    );
}

// ---------------------------------------------------------------------------
// record-human-turn — 記録・カーソルが無いときは何も書かない
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_human_turn_without_a_record_or_a_cursor_writes_nothing() {
    let workspace = Workspace::new();
    let none = workspace.hook(
        "record-human-turn",
        r#"{"session_id":"nobody","prompt":"hello"}"#,
        &[],
    );
    assert_eq!(none.status.code(), Some(0), "{none:?}");
    assert!(!workspace.intents().join(".aidlc-store.sqlite").exists());
    workspace.mint("bugfix", "cursorless").await;
    fs::remove_file(workspace.record().join(".aidlc-execution")).unwrap();
    let before = workspace.audit();
    let output = workspace.hook(
        "record-human-turn",
        r#"{"session_id":"nobody","prompt":"hello"}"#,
        &[],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(workspace.audit(), before);
}

#[tokio::test]
async fn a_refused_hook_drop_record_does_not_stop_the_hook() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "dropless").await;
    let first = workspace.invoke("aidlc-orchestrate", &["next"]).await;
    assert_eq!(first.code(), 0, "{first:?}");
    let lock = workspace.root().join("aidlc/.aidlc-runtime.lock");
    let _ = fs::remove_file(&lock);
    fs::create_dir(&lock).unwrap();
    let db = workspace.runtime_db();
    db.execute_batch("CREATE TRIGGER fail_drop BEFORE INSERT ON journal WHEN NEW.manifest='hook-health-event/1' AND CAST(NEW.payload AS TEXT) LIKE '%rop%' BEGIN SELECT RAISE(ABORT,'injected drop failure'); END;").unwrap();
    let output = workspace.hook("continue-workflow", r#"{"stop_hook_active":false}"#, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(
        !drops(&workspace).contains("Is a directory"),
        "拒まれた drop は描かれない: {}",
        drops(&workspace)
    );
}

// ---------------------------------------------------------------------------
// aidlc-log answer / decision (plan-approval) — 計画文書とカーソルの前提
// ---------------------------------------------------------------------------

#[tokio::test]
async fn plan_approval_verbs_refuse_a_malformed_unit_a_missing_plan_and_a_broken_cursor() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "plan").await;
    let decision = |extra: Vec<&'static str>| -> Vec<&'static str> {
        let mut args = vec![
            "decision",
            "--stage",
            "code-generation",
            "--checkpoint",
            "plan-approval",
            "--session",
            "stage1-session",
            "--decision",
            "Approve this exact Code Generation plan?",
            "--options",
            "Approve Plan,Request Changes",
        ];
        args.extend(extra);
        args
    };
    let answer = |extra: Vec<&'static str>| -> Vec<&'static str> {
        let mut args = vec![
            "answer",
            "--stage",
            "code-generation",
            "--checkpoint",
            "plan-approval",
            "--session",
            "stage1-session",
            "--details",
            "Approve Plan",
        ];
        args.extend(extra);
        args
    };
    let unit = "Invalid Unit name \"Bad Unit!\" - must match /^[A-Za-z0-9][A-Za-z0-9._-]*$/ (ASCII letter/digit, then ASCII letters/digits/dot/underscore/hyphen)";
    let bad_unit = workspace
        .invoke("aidlc-log", &decision(vec!["--unit", "Bad Unit!"]))
        .await;
    assert_eq!(refused_error(&bad_unit), unit);
    let bad_unit = workspace
        .invoke("aidlc-log", &answer(vec!["--unit", "Bad Unit!"]))
        .await;
    assert_eq!(refused_error(&bad_unit), unit);
    // 計画文書が無い記録では参照入力を読めない。
    let no_plan = workspace
        .invoke("aidlc-log", &decision(vec!["--stage-level"]))
        .await;
    assert_eq!(no_plan.code(), 1, "{no_plan:?}");
    let no_plan_answer = workspace
        .invoke("aidlc-log", &answer(vec!["--stage-level"]))
        .await;
    assert_eq!(no_plan_answer.code(), 1, "{no_plan_answer:?}");
    assert_eq!(
        refused_error(&no_plan_answer),
        refused_error(&no_plan),
        "同じ参照入力の欠落を同じ文言で断る"
    );
    // 壊れたカーソルは未鋳造と混ぜない。
    workspace.break_cursor();
    let broken = workspace
        .invoke("aidlc-log", &decision(vec!["--stage-level"]))
        .await;
    assert!(
        refused_error(&broken).contains("garbage") || refused_error(&broken).contains("cursor"),
        "{broken:?}"
    );
    let broken_answer = workspace
        .invoke("aidlc-log", &answer(vec!["--stage-level"]))
        .await;
    assert_eq!(refused_error(&broken_answer), refused_error(&broken));
}

// ---------------------------------------------------------------------------
// sync-workflow-state — 稼働記録の拒否・カーソル無し・保存拒否
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sync_workflow_state_stops_at_a_refused_heartbeat_and_skips_without_a_cursor() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "synclock").await;
    let input = r#"{"tool_name":"TaskUpdate","tool_input":{"taskId":"1","status":"in_progress","activeForm":"Running [reverse-engineering]"}}"#;
    // 承認ストアの初期化印が壊れていれば稼働記録が断り、その拒否をそのまま返す。
    let marker = workspace.root().join("aidlc/.aidlc-runtime.state.json");
    fs::write(&marker, "{").unwrap();
    let refused = workspace.hook("sync-workflow-state", input, &[]);
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    assert_eq!(
        String::from_utf8_lossy(&refused.stderr).trim(),
        "Invalid shared approval initialization marker"
    );
    fs::remove_file(&marker).unwrap();
    // カーソルが無い記録は同期しない（沈黙）。
    fs::remove_file(workspace.record().join(".aidlc-execution")).unwrap();
    let silent = workspace.hook("sync-workflow-state", input, &[]);
    assert_eq!(silent.status.code(), Some(0), "{silent:?}");
    assert!(
        silent.stdout.is_empty() && silent.stderr.is_empty(),
        "{silent:?}"
    );
}

// ---------------------------------------------------------------------------
// 壊れた読み面 — 列を失った投影表は「読めない」と名指しで断る（推測しない）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn next_refuses_a_read_model_whose_projection_tables_lost_their_columns() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "columns").await;
    let first = workspace.invoke("aidlc-orchestrate", &["next"]).await;
    assert_eq!(first.code(), 0, "{first:?}");
    let store = workspace.intents().join(".aidlc-store.sqlite");
    let expected_prefix = format!("Read model not readable at {}: ", store.display());
    for (table, column, args) in [
        ("read_intent", "project_type", vec!["next"]),
        (
            "read_scope_change",
            "kind",
            vec!["next", "--scope", "classic"],
        ),
        ("read_pipeline_progress", "completed", vec!["next"]),
        ("read_steering_plan", "bundle_digest", vec!["next"]),
        ("read_run_stage", "stage_slug", vec!["next"]),
        ("read_next_answer", "request_kind", vec!["next"]),
        ("read_execution", "intent_id", vec!["next"]),
    ] {
        let db = workspace.store_db();
        let renamed = db.execute_batch(&format!(
            "ALTER TABLE {table} RENAME COLUMN {column} TO {column}_lost"
        ));
        assert!(renamed.is_ok(), "{table}.{column}: {renamed:?}");
        drop(db);
        let refused = next_error(&workspace.invoke("aidlc-orchestrate", &args).await);
        assert!(
            refused.starts_with(&expected_prefix),
            "{table}.{column}: {refused}"
        );
        let db = workspace.store_db();
        db.execute_batch(&format!(
            "ALTER TABLE {table} RENAME COLUMN {column}_lost TO {column}"
        ))
        .unwrap();
    }
    let restored = workspace.invoke("aidlc-orchestrate", &["next"]).await;
    assert_eq!(restored.code(), 0, "{restored:?}");
    assert_ne!(
        emitted_json(&restored).get("kind").unwrap(),
        "error",
        "{restored:?}"
    );
    // 定義の段の行そのものを失えば、その欠落を名指しで断る。
    workspace
        .store_db()
        .execute_batch("DELETE FROM read_definition_stage WHERE stage_slug='reverse-engineering'")
        .unwrap();
    let refused = next_error(&workspace.invoke("aidlc-orchestrate", &["next"]).await);
    assert_eq!(refused, "stage metadata unavailable");
}

#[tokio::test]
async fn query_verbs_refuse_projection_tables_that_lost_their_columns() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "querycols").await;
    let first = workspace.invoke("aidlc-orchestrate", &["next"]).await;
    assert_eq!(first.code(), 0, "{first:?}");
    let contract = workspace.invoke("aidlc-testing-posture", &["render"]).await;
    assert_eq!(contract.code(), 0, "{contract:?}");
    let store = workspace.intents().join(".aidlc-store.sqlite");
    let sabotage = |table: &str, column: &str| {
        workspace
            .store_db()
            .execute_batch(&format!(
                "ALTER TABLE {table} RENAME COLUMN {column} TO {column}_lost"
            ))
            .unwrap();
    };
    sabotage("read_next_jump", "resolution");
    let stage = workspace
        .invoke("aidlc-jump", &["resolve", "--stage", "intent-capture"])
        .await;
    assert!(
        refused_error(&stage).contains(&store.display().to_string()),
        "{stage:?}"
    );
    sabotage("read_next_jump_phase", "target_slug");
    let phase = workspace
        .invoke("aidlc-jump", &["resolve", "--phase", "ideation"])
        .await;
    assert!(
        refused_error(&phase).contains(&store.display().to_string()),
        "{phase:?}"
    );
    sabotage("read_testing_contract", "contract");
    let posture = workspace.invoke("aidlc-testing-posture", &["render"]).await;
    assert!(
        posture_error(&posture).contains(&store.display().to_string()),
        "{posture:?}"
    );
}

// ---------------------------------------------------------------------------
// continue-workflow (Stop) — 共有 resume 待ちの証拠があってもカーソルが壊れていれば通す
// ---------------------------------------------------------------------------

/// 現在の状態に束ねられた共有 resume 待ちの印を書く（`stop_preserves_a_state_bound_shared_resume_prompt` と同形）。
fn write_state_bound_resume_wait(workspace: &Workspace) {
    let marker_path = workspace.record().join(".aidlc-active-directive.json");
    let mut marker: serde_json::Value =
        serde_json::from_slice(&fs::read(&marker_path).unwrap()).unwrap();
    let state_hash = core_infrastructure::hash::sha256_hex(workspace.state().as_bytes());
    let fields = marker.as_object_mut().unwrap();
    fields.insert("kind".into(), serde_json::Value::String("ask".into()));
    fields.insert(
        "state_sha256".into(),
        serde_json::Value::String(state_hash.clone()),
    );
    let resume = serde_json::Value::Object(serde_json::Map::from_iter([
        ("status".into(), serde_json::Value::String("waiting".into())),
        (
            "issuing_stage".into(),
            serde_json::Value::String("reverse-engineering".into()),
        ),
        (
            "issuing_state_sha256".into(),
            serde_json::Value::String(state_hash),
        ),
        (
            "issuing_session".into(),
            serde_json::Value::String("sessionless:scope".into()),
        ),
        ("issuing_intent_uuid".into(), serde_json::Value::Null),
    ]));
    fields.insert("resume".into(), resume);
    fs::write(
        &marker_path,
        core_infrastructure::canon_json::serialize(
            &core_infrastructure::canon_json::to_value(&marker).unwrap(),
            core_infrastructure::canon_json::SerializationProfile::ContractPretty,
        ),
    )
    .unwrap();
}

#[tokio::test]
async fn a_resume_wait_cannot_be_recorded_against_a_broken_cursor() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "resumewait").await;
    let first = workspace.invoke("aidlc-orchestrate", &["next"]).await;
    assert_eq!(first.code(), 0, "{first:?}");
    write_state_bound_resume_wait(&workspace);
    let waiting = workspace.hook("continue-workflow", "{}", &[]);
    assert_eq!(waiting.status.code(), Some(0), "{waiting:?}");
    assert!(waiting.stdout.is_empty(), "待ちは沈黙: {waiting:?}");
    assert!(
        drops(&workspace).contains("active resume choice is waiting on the human"),
        "{}",
        drops(&workspace)
    );
    workspace.break_cursor();
    let broken = workspace.hook("continue-workflow", "{}", &[]);
    assert_eq!(broken.status.code(), Some(0), "{broken:?}");
    assert!(broken.stdout.is_empty(), "{broken:?}");
    assert!(
        drops(&workspace).contains("garbage") || drops(&workspace).contains("cursor"),
        "{}",
        drops(&workspace)
    );
}

// ---------------------------------------------------------------------------
// pipeline link — 在るのに読めない handoff は「安全に読める通常ファイル」ではない
// ---------------------------------------------------------------------------

#[tokio::test]
async fn an_unreadable_developer_handoff_is_refused_with_its_cause() {
    use std::os::unix::fs::PermissionsExt as _;
    let workspace = Workspace::new();
    workspace.mint("bugfix", "handoff").await;
    let handoff = workspace
        .record()
        .join("inception/reverse-engineering/developer-scan.md");
    fs::create_dir_all(handoff.parent().unwrap()).unwrap();
    fs::write(&handoff, "## Developer Code Scan Results\n").unwrap();
    fs::set_permissions(&handoff, fs::Permissions::from_mode(0o000)).unwrap();
    let relative = handoff
        .strip_prefix(workspace.root())
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let linked = workspace
        .invoke(
            "aidlc-log",
            &[
                "link",
                "--stage",
                "reverse-engineering",
                "--link",
                "aidlc-developer-agent",
                "--artifact",
                &relative,
            ],
        )
        .await;
    fs::set_permissions(&handoff, fs::Permissions::from_mode(0o644)).unwrap();
    let message = refused_error(&linked);
    assert!(
        message.starts_with("Cannot record reverse-engineering developer link: handoff file must be a regular file with no symlink path components (")
            && message.contains("Permission denied"),
        "{message}"
    );
}

// ---------------------------------------------------------------------------
// review-freeze — 文法外の space では判断を下さず通す
// ---------------------------------------------------------------------------

#[tokio::test]
async fn review_freeze_opens_in_an_invalid_space_without_a_verdict() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "freezespace").await;
    let record = workspace.record();
    let bad = workspace.move_into_an_invalid_space();
    let target = workspace
        .root()
        .join("aidlc/spaces")
        .join(&bad)
        .join("intents")
        .join(record.file_name().unwrap())
        .join("inception/requirements-analysis/requirements.md");
    let input = format!(
        r#"{{"tool_name":"Write","tool_input":{{"file_path":"{}"}}}}"#,
        target.display()
    );
    let output = workspace.hook("review-freeze", &input, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
}

// ---------------------------------------------------------------------------
// rebuild-stage-graph — コンパイルの失敗は drop に残して通す
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rebuild_stage_graph_drops_a_broken_cursor_and_an_unreadable_store() {
    let workspace = Workspace::new();
    workspace.mint("bugfix", "rebuild").await;
    let record = workspace.record();
    fs::create_dir_all(record.join("inception/requirements-analysis")).unwrap();
    fs::write(
        record.join("inception/requirements-analysis/memory.md"),
        "# Memory\n",
    )
    .unwrap();
    let compiled = workspace.hook("rebuild-stage-graph", COMPILE_INPUT, &[]);
    assert_eq!(compiled.status.code(), Some(0), "{compiled:?}");
    assert!(record.join("runtime-graph.json").is_file());
    let before = fs::read(record.join("runtime-graph.json")).unwrap();
    workspace.break_cursor();
    let broken = workspace.hook("rebuild-stage-graph", COMPILE_INPUT, &[]);
    assert_eq!(broken.status.code(), Some(0), "{broken:?}");
    let drops = workspace.hook_drops("rebuild-stage-graph");
    assert!(
        drops.contains("garbage") || drops.contains("cursor"),
        "{drops}"
    );
    fs::write(record.join(".aidlc-execution"), "").unwrap();
    // カーソルを戻せないので、壊れたストアは別の記録で観測する。
    let other = Workspace::new();
    other.mint("bugfix", "rebuild2").await;
    let other_record = other.record();
    fs::create_dir_all(other_record.join("inception/requirements-analysis")).unwrap();
    fs::write(
        other_record.join("inception/requirements-analysis/memory.md"),
        "# Memory\n",
    )
    .unwrap();
    other.corrupt_store();
    let corrupt = other.hook("rebuild-stage-graph", COMPILE_INPUT, &[]);
    assert_eq!(corrupt.status.code(), Some(0), "{corrupt:?}");
    assert!(
        other
            .hook_drops("rebuild-stage-graph")
            .contains("InvalidData"),
        "{}",
        other.hook_drops("rebuild-stage-graph")
    );
    assert_eq!(fs::read(record.join("runtime-graph.json")).unwrap(), before);
}

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;

#[path = "../../../../tests/support/tool_link.rs"]
mod tool_link;
