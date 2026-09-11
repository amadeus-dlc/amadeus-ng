//! レビュー保護フック（review-freeze / reviewer-scope）を公開プロセス面で検証する。
#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::{
    fs,
    io::Write as _,
    path::PathBuf,
    process::{Command, Output, Stdio},
};

struct Workspace {
    temp: tempfile::TempDir,
}
impl Workspace {
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
        fs::copy(
            repository.join(".claude/scopes/aidlc-bugfix.md"),
            root.join(".claude/scopes/aidlc-bugfix.md"),
        )
        .unwrap();
        for tool in ["aidlc-utility", "aidlc-log", "aidlc"] {
            tool_link::link_tool(&temp.path().join(tool)).unwrap();
        }
        fs::create_dir_all(root.join("aidlc/spaces/default/intents")).unwrap();
        fs::write(root.join("source.rs"), "fn main() {}\n").unwrap();
        let workspace = Self { temp };
        let created = workspace.run(
            "aidlc-utility",
            &[
                "intent-create",
                "--scope",
                "bugfix",
                "--label",
                "guards",
                "--arguments",
                "Fix the review guards",
            ],
            &[],
        );
        assert!(created.status.success(), "{created:?}");
        workspace
    }
    fn root(&self) -> PathBuf {
        self.temp.path().join("workspace")
    }
    fn run(&self, tool: &str, args: &[&str], env: &[(&str, &str)]) -> Output {
        let mut command = Command::new(self.temp.path().join(tool));
        command
            .args(args)
            .current_dir(self.root())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.temp.path().join("home"))
            .env("PATH", "/usr/bin:/bin");
        for (key, value) in env {
            command.env(key, value);
        }
        command.output().unwrap()
    }
    fn hook(&self, name: &str, stdin: &str, env: &[(&str, &str)]) -> Output {
        let mut command = Command::new(self.temp.path().join("aidlc"));
        command
            .args(["hook", name])
            .current_dir(self.root())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.temp.path().join("home"))
            .env("PATH", "/usr/bin:/bin")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in env {
            command.env(key, value);
        }
        let mut child = command.spawn().unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    }
    fn record(&self) -> PathBuf {
        let intents = self.root().join("aidlc/spaces/default/intents");
        intents.join(
            fs::read_to_string(intents.join("active-intent"))
                .unwrap()
                .trim(),
        )
    }
    fn audit(&self) -> String {
        let mut paths = fs::read_dir(self.record().join("audit"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        paths.sort();
        paths
            .into_iter()
            .map(|path| fs::read_to_string(path).unwrap())
            .collect()
    }
    /// U1 が採取した本家の requirements 一式を記録へ置く。
    fn requirements(&self, completed: bool) {
        use base64::Engine as _;
        let corpus: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/golden/upstream-a277af21/stage1/cases.json"
        ))
        .unwrap();
        let id = if completed {
            "review/completed"
        } else {
            "review/request"
        };
        let case = corpus
            .get("observations")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case.get("id").and_then(serde_json::Value::as_str) == Some(id))
            .unwrap();
        for (path, encoded) in case.get("initial_files").unwrap().as_object().unwrap() {
            if let Some((_, relative)) = path.split_once("/inception/requirements-analysis/") {
                let destination = self
                    .record()
                    .join("inception/requirements-analysis")
                    .join(relative);
                fs::create_dir_all(destination.parent().unwrap()).unwrap();
                fs::write(
                    destination,
                    base64::engine::general_purpose::STANDARD
                        .decode(encoded.as_str().unwrap())
                        .unwrap(),
                )
                .unwrap();
            }
        }
    }
    /// requirements-analysis に終端の受領証（advisory の READY）を積む。
    fn terminal_receipt(&self) {
        self.requirements(false);
        let requested = self.run(
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
            &[],
        );
        assert!(requested.status.success(), "{requested:?}");
        self.requirements(true);
        let completed = self.run(
            "aidlc-log",
            &[
                "review",
                "--stage",
                "requirements-analysis",
                "--reviewer",
                "aidlc-product-lead-agent",
                "--iteration",
                "1",
                "--verdict",
                "READY",
            ],
            &[],
        );
        assert!(completed.status.success(), "{completed:?}");
    }
    fn write_input(&self, relative: &str) -> String {
        tool_input("Write", None, "file_path", &self.path_text(relative))
    }
    fn path_text(&self, relative: &str) -> String {
        self.record().join(relative).to_string_lossy().into_owned()
    }
}

/// フックの標準入力 1 件を組む。契約 JSON ではなくハーネス入力の再現なので、
/// 逐語の文字列として書く (`clippy.toml` は契約直列化を canon-json に固定している)。
fn tool_input(tool: &str, agent: Option<&str>, key: &str, value: &str) -> String {
    let agent = agent.map_or_else(String::new, |agent| {
        format!(r#""agent_type":"{}","#, escape(agent))
    });
    format!(
        r#"{{"tool_name":"{}",{}"tool_input":{{"{}":"{}"}}}}"#,
        escape(tool),
        agent,
        escape(key),
        escape(value)
    )
}

/// `Bash` の PreToolUse 入力 1 件を組む。`cwd` を省くと呼出しの作業ディレクトリが基点になる。
fn shell_input(command: &str, cwd: Option<&str>) -> String {
    let cwd = cwd.map_or_else(String::new, |cwd| format!(r#""cwd":"{}","#, escape(cwd)));
    format!(
        r#"{{"tool_name":"Bash",{}"tool_input":{{"command":"{}"}}}}"#,
        cwd,
        escape(command)
    )
}

/// JSON 文字列に置ける形へ逃がす (テスト入力に現れるのは `\` と `"` だけ)。
fn escape(raw: &str) -> String {
    raw.replace('\\', "\\\\").replace('"', "\\\"")
}

#[test]
fn a_declared_artifact_write_is_refused_and_recorded_while_the_receipt_stands() {
    let workspace = Workspace::new();
    workspace.terminal_receipt();
    let output = workspace.hook(
        "review-freeze",
        &workspace.write_input("inception/requirements-analysis/requirements.md"),
        &[],
    );
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("review-freeze: "),
        "拒否理由は逐語で始まる: {stderr}"
    );
    assert!(
        stderr.contains("stage \"requirements-analysis\""),
        "{stderr}"
    );
    assert!(
        stderr.contains("quote it at the gate instead of applying it."),
        "{stderr}"
    );
    assert!(output.stdout.is_empty(), "拒否は stdout を汚さない");
    let audit = workspace.audit();
    assert!(audit.contains("## Review Freeze Blocked"), "{audit}");
    assert!(
        audit.contains("**Event**: REVIEW_FREEZE_BLOCKED"),
        "{audit}"
    );
    assert!(audit.contains("**Tool**: Write"), "{audit}");
    assert!(
        audit.contains("**Target**: ")
            && audit.contains("inception/requirements-analysis/requirements.md"),
        "{audit}"
    );
    assert!(
        audit.contains("**Stage**: requirements-analysis"),
        "{audit}"
    );
    assert!(
        !audit.contains("**Unit**: "),
        "ゼロ Unit の書込みは Unit を名乗らない"
    );
}

#[test]
fn writes_outside_the_declared_artifacts_pass_untouched() {
    let workspace = Workspace::new();
    workspace.terminal_receipt();
    for relative in [
        "inception/requirements-analysis/memory.md",
        "construction/code-generation/code-generation-plan.md",
    ] {
        let output = workspace.hook("review-freeze", &workspace.write_input(relative), &[]);
        assert_eq!(output.status.code(), Some(0), "{relative}: {output:?}");
    }
    assert!(!workspace.audit().contains("REVIEW_FREEZE_BLOCKED"));
}

#[test]
fn a_read_only_call_and_malformed_input_are_allowed_without_a_record() {
    let workspace = Workspace::new();
    workspace.terminal_receipt();
    let read = tool_input(
        "Read",
        None,
        "file_path",
        &workspace.path_text("inception/requirements-analysis/requirements.md"),
    );
    for input in [read.as_str(), "not json", "{}", "[]"] {
        let output = workspace.hook("review-freeze", input, &[]);
        assert_eq!(output.status.code(), Some(0), "{input}: {output:?}");
    }
    assert!(!workspace.audit().contains("REVIEW_FREEZE_BLOCKED"));
}

#[test]
fn the_freeze_does_not_bite_before_the_verdict_lands() {
    let workspace = Workspace::new();
    workspace.requirements(false);
    let requested = workspace.run(
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
        &[],
    );
    assert!(requested.status.success(), "{requested:?}");
    let output = workspace.hook(
        "review-freeze",
        &workspace.write_input("inception/requirements-analysis/requirements.md"),
        &[],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
}

#[test]
fn the_documented_off_switch_disables_enforcement_entirely() {
    let workspace = Workspace::new();
    workspace.terminal_receipt();
    let output = workspace.hook(
        "review-freeze",
        &workspace.write_input("inception/requirements-analysis/requirements.md"),
        &[("AIDLC_DISABLE_REVIEW_FREEZE_HOOK", "1")],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(!workspace.audit().contains("REVIEW_FREEZE_BLOCKED"));
}

#[test]
fn the_freeze_hook_records_its_heartbeat() {
    let workspace = Workspace::new();
    workspace.terminal_receipt();
    let output = workspace.hook("review-freeze", "{}", &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let heartbeat = workspace
        .record()
        .join(".aidlc-hooks-health/review-freeze.last");
    assert!(heartbeat.is_file(), "稼働記録が残る: {heartbeat:?}");
}

#[test]
fn the_reviewer_scope_hook_records_its_heartbeat_and_advises_once_on_a_missing_record() {
    let workspace = Workspace::new();
    let input = tool_input(
        "Read",
        Some("aidlc-architecture-reviewer-agent"),
        "file_path",
        &workspace.path_text("construction/u1-x/functional-design/entities.md"),
    );
    for _ in 0..2 {
        let output = workspace.hook("reviewer-scope", &input, &[]);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
    }
    let health = workspace.record().join(".aidlc-hooks-health");
    assert!(health.join("reviewer-scope.last").is_file());
    assert!(
        health.join("reviewer-scope.missing-record.last").is_file(),
        "記録不在の助言は目印を残す"
    );
    let drops = fs::read_to_string(health.join("reviewer-scope.drops")).unwrap_or_default();
    assert_eq!(
        drops.matches("reviewer dispatch record").count(),
        1,
        "助言は 10 分に 1 度に抑える: {drops}"
    );
}

#[test]
fn a_non_reviewer_call_leaves_no_advisory() {
    let workspace = Workspace::new();
    let input = tool_input(
        "Read",
        Some("aidlc-developer-agent"),
        "file_path",
        "construction/u1-x/functional-design/entities.md",
    );
    let output = workspace.hook("reviewer-scope", &input, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(
        !workspace
            .record()
            .join(".aidlc-hooks-health/reviewer-scope.missing-record.last")
            .exists(),
        "レビュー専用エージェント以外は助言の対象ではない"
    );
}

#[test]
fn the_reviewer_scope_off_switch_stops_every_observation() {
    let workspace = Workspace::new();
    let input = tool_input(
        "Read",
        Some("aidlc-product-lead-agent"),
        "file_path",
        "construction/u1-x/functional-design/entities.md",
    );
    let output = workspace.hook(
        "reviewer-scope",
        &input,
        &[("AIDLC_DISABLE_REVIEWER_SCOPE_HOOK", "1")],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(
        !workspace
            .record()
            .join(".aidlc-hooks-health/reviewer-scope.last")
            .exists()
    );
}

/// 差し向け記録 1 件を置く。`exempt` は upstream `parseDispatchRecord` が必須にする。
fn write_dispatch(workspace: &Workspace, exempt: &[&str]) {
    let exempt = exempt
        .iter()
        .map(|path| format!("\"{}\"", escape(path)))
        .collect::<Vec<_>>()
        .join(",");
    fs::write(
        workspace.record().join(".aidlc-reviewer-dispatch.json"),
        format!(
            "{{\"reviewer\":\"aidlc-architecture-reviewer-agent\",\"stage\":\"functional-design\",\"unit\":\"u1-x\",\"exempt\":[{exempt}]}}\n"
        ),
    )
    .unwrap();
}

/// 差し向けられたレビュアーによる Read 入力 1 件。
fn reviewer_read(workspace: &Workspace, relative: &str) -> String {
    tool_input(
        "Read",
        Some("aidlc-architecture-reviewer-agent"),
        "file_path",
        &workspace.path_text(relative),
    )
}

/// フックが残した drop の現物。
fn drops(workspace: &Workspace) -> String {
    fs::read_to_string(
        workspace
            .record()
            .join(".aidlc-hooks-health/reviewer-scope.drops"),
    )
    .unwrap_or_default()
}

#[test]
fn a_dispatched_reviewer_reaching_a_sibling_unit_is_refused_and_recorded() {
    let workspace = Workspace::new();
    write_dispatch(&workspace, &[]);
    // 監査行の `Target` は project dir を `<project-dir>` へ伏せる (upstream `renderAuditBlock`
    // は全値に `redactProjectDirPrefix` を掛ける — hook-differential c01)。フックの project dir
    // は作業ディレクトリの実パスなので、入力もその綴りで組んで置換の観測を確定させる。
    let record = fs::canonicalize(workspace.record()).unwrap();
    let sibling = record
        .join("construction/u2-y/functional-design/entities.md")
        .to_string_lossy()
        .into_owned();
    let output = workspace.hook(
        "reviewer-scope",
        &tool_input(
            "Read",
            Some("aidlc-architecture-reviewer-agent"),
            "file_path",
            &sibling,
        ),
        &[],
    );
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("This review cannot open \""),
        "拒否理由は逐語で始まる: {stderr}"
    );
    assert!(
        stderr.contains("the current review covers u1-x."),
        "{stderr}"
    );
    assert!(
        stderr.contains("keep searches inside the current unit."),
        "{stderr}"
    );
    assert!(output.stdout.is_empty(), "拒否は stdout を汚さない");
    let audit = workspace.audit();
    assert!(audit.contains("## Reviewer Scope Blocked"), "{audit}");
    assert!(
        audit.contains("**Event**: REVIEWER_SCOPE_BLOCKED"),
        "{audit}"
    );
    assert!(audit.contains("**Tool**: Read"), "{audit}");
    let expected_target = format!(
        "**Target**: <project-dir>/aidlc/spaces/default/intents/{}/construction/u2-y/functional-design/entities.md\n",
        record.file_name().unwrap().to_string_lossy()
    );
    assert!(
        audit.contains(&expected_target),
        "監査の Target は project dir を伏せる: {audit}"
    );
    assert!(
        !audit.contains(&sibling),
        "生の絶対経路は監査に残らない: {audit}"
    );
    assert!(audit.contains("**Stage**: functional-design"), "{audit}");
    assert!(audit.contains("**Unit**: u1-x"), "{audit}");
}

#[test]
fn the_current_unit_and_an_exempt_sibling_file_stay_open() {
    let workspace = Workspace::new();
    let exempt = workspace.path_text("construction/u2-y/functional-design/entities.md");
    write_dispatch(&workspace, &[&exempt]);
    for relative in [
        "construction/u1-x/functional-design/entities.md",
        "construction/u2-y/functional-design/entities.md",
        "inception/requirements-analysis/requirements.md",
    ] {
        let output = workspace.hook("reviewer-scope", &reviewer_read(&workspace, relative), &[]);
        assert_eq!(output.status.code(), Some(0), "{relative}: {output:?}");
    }
    assert!(
        !workspace.audit().contains("REVIEWER_SCOPE_BLOCKED"),
        "許可に監査行は出ない"
    );
}

#[test]
fn the_exempt_carve_out_is_exact_so_browsing_its_directory_still_crosses() {
    let workspace = Workspace::new();
    let exempt = workspace.path_text("construction/u2-y/functional-design/entities.md");
    write_dispatch(&workspace, &[&exempt]);
    let output = workspace.hook(
        "reviewer-scope",
        &tool_input(
            "LS",
            Some("aidlc-architecture-reviewer-agent"),
            "path",
            &workspace.path_text("construction/u2-y/functional-design"),
        ),
        &[],
    );
    assert_eq!(output.status.code(), Some(2), "{output:?}");
}

#[test]
fn a_pathless_search_that_would_sweep_every_sibling_is_refused() {
    let workspace = Workspace::new();
    write_dispatch(&workspace, &[]);
    let output = workspace.hook(
        "reviewer-scope",
        r#"{"tool_name":"Grep","agent_type":"aidlc-architecture-reviewer-agent","tool_input":{"pattern":"anything"}}"#,
        &[],
    );
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        workspace.audit().contains("**Target**: ."),
        "掃く根は綴りのまま監査へ載る: {}",
        workspace.audit()
    );
}

#[test]
fn a_glob_that_sweeps_the_siblings_is_refused_while_one_scoped_to_the_unit_passes() {
    // parity の case-042 / 043 と実走行 c06 / c07 の子プロセス面。`Glob` の `pattern` は経路の形を
    // したパターンとして読まれ、兄弟を掃く形なら拒否、現 Unit へ絞った形なら通す。
    let workspace = Workspace::new();
    write_dispatch(&workspace, &[]);
    let sweeping = workspace.hook(
        "reviewer-scope",
        &tool_input(
            "Glob",
            Some("aidlc-architecture-reviewer-agent"),
            "pattern",
            "construction/*/design.md",
        ),
        &[],
    );
    assert_eq!(sweeping.status.code(), Some(2), "{sweeping:?}");
    assert!(
        String::from_utf8_lossy(&sweeping.stderr)
            .starts_with("This review cannot open \"construction/*/design.md\""),
        "{sweeping:?}"
    );
    let audit = workspace.audit();
    assert!(audit.contains("**Tool**: Glob"), "{audit}");
    assert!(
        audit.contains("**Target**: construction/*/design.md\n"),
        "{audit}"
    );
    let scoped = workspace.hook(
        "reviewer-scope",
        &tool_input(
            "Glob",
            Some("aidlc-architecture-reviewer-agent"),
            "pattern",
            "construction/u1-x/**/*.md",
        ),
        &[],
    );
    assert_eq!(scoped.status.code(), Some(0), "{scoped:?}");
    assert_eq!(
        workspace.audit().matches("REVIEWER_SCOPE_BLOCKED").count(),
        1,
        "現 Unit へ絞った Glob は監査行を増やさない"
    );
}

#[test]
fn malformed_stdin_passes_the_reviewer_scope_hook_after_its_heartbeat() {
    // 実走行 c20 / c21 / c31: 標準入力が JSON でない・object でない・空のときは、記録が
    // 在っても素通しする。稼働記録は判断より先に残る。
    let workspace = Workspace::new();
    write_dispatch(&workspace, &[]);
    for stdin in ["not json", "[]", ""] {
        let output = workspace.hook("reviewer-scope", stdin, &[]);
        assert_eq!(output.status.code(), Some(0), "{stdin:?}: {output:?}");
        assert!(
            output.stdout.is_empty() && output.stderr.is_empty(),
            "{output:?}"
        );
    }
    assert!(
        workspace
            .record()
            .join(".aidlc-hooks-health/reviewer-scope.last")
            .is_file(),
        "稼働記録は入力の形に関わらず残る"
    );
    assert!(!workspace.audit().contains("REVIEWER_SCOPE_BLOCKED"));
    assert!(
        workspace
            .record()
            .join(".aidlc-reviewer-dispatch.json")
            .is_file(),
        "読めない入力は差し向け記録を消さない"
    );
}

#[test]
fn a_shell_call_reaching_a_sibling_unit_is_refused_through_the_same_path() {
    let workspace = Workspace::new();
    write_dispatch(&workspace, &[]);
    let command = format!(
        "cat {}",
        workspace.path_text("construction/u2-y/functional-design/entities.md")
    );
    let output = workspace.hook(
        "reviewer-scope",
        &tool_input(
            "Bash",
            Some("aidlc-architecture-reviewer-agent"),
            "command",
            &command,
        ),
        &[],
    );
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        workspace.audit().contains("**Tool**: Bash"),
        "{}",
        workspace.audit()
    );
}

#[test]
fn only_the_dispatched_reviewer_is_enforced_against() {
    let workspace = Workspace::new();
    write_dispatch(&workspace, &[]);
    let sibling = workspace.path_text("construction/u2-y/functional-design/entities.md");
    // 指揮者自身 (名乗らない) と別のサブエージェントは素通しする。
    for agent in [
        None,
        Some("aidlc-developer-agent"),
        Some("aidlc-product-lead-agent"),
    ] {
        let output = workspace.hook(
            "reviewer-scope",
            &tool_input("Read", agent, "file_path", &sibling),
            &[],
        );
        assert_eq!(output.status.code(), Some(0), "{agent:?}: {output:?}");
    }
    assert!(!workspace.audit().contains("REVIEWER_SCOPE_BLOCKED"));
}

#[test]
fn a_malformed_dispatch_record_skips_enforcement_and_records_why() {
    let workspace = Workspace::new();
    // `exempt` の無い記録は upstream `parseDispatchRecord` も拒否する。
    fs::write(
        workspace.record().join(".aidlc-reviewer-dispatch.json"),
        "{\"reviewer\":\"aidlc-architecture-reviewer-agent\",\"stage\":\"functional-design\",\"unit\":\"u1-x\"}\n",
    )
    .unwrap();
    let output = workspace.hook(
        "reviewer-scope",
        &reviewer_read(
            &workspace,
            "construction/u2-y/functional-design/entities.md",
        ),
        &[],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(
        drops(&workspace).contains("reviewer dispatch record is malformed"),
        "黙って通さない: {}",
        drops(&workspace)
    );
    assert!(!workspace.audit().contains("REVIEWER_SCOPE_BLOCKED"));
}

#[test]
fn a_dispatch_record_is_enforced_with_its_stage_and_unit_spelled_verbatim() {
    // upstream `parseDispatchRecord` は `stage` の文法も `unit` の区切りも見ない。固定 2.7.1 の
    // 実走行 (hook-differential c24 / c25) は `stage: "Functional Design"` と `unit: "a/b"` を
    // そのまま強制し、`Stage` / `Unit` を監査へ逐語で載せた (裁定 Q1 = A)。
    let sibling = "construction/u2-y/functional-design/entities.md";
    for (record, expected_row, expected_reason) in [
        (
            "{\"reviewer\":\"aidlc-architecture-reviewer-agent\",\"stage\":\"Functional Design\",\"unit\":\"u1-x\",\"exempt\":[]}\n",
            "**Stage**: Functional Design\n",
            "the current review covers u1-x.",
        ),
        (
            "{\"reviewer\":\"aidlc-architecture-reviewer-agent\",\"stage\":\"functional-design\",\"unit\":\"a/b\",\"exempt\":[]}\n",
            "**Unit**: a/b\n",
            "the current review covers a/b.",
        ),
    ] {
        let workspace = Workspace::new();
        fs::write(
            workspace.record().join(".aidlc-reviewer-dispatch.json"),
            record,
        )
        .unwrap();
        let output = workspace.hook("reviewer-scope", &reviewer_read(&workspace, sibling), &[]);
        assert_eq!(output.status.code(), Some(2), "{record}: {output:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(expected_reason), "{record}: {stderr}");
        let audit = workspace.audit();
        assert!(
            audit.contains("**Event**: REVIEWER_SCOPE_BLOCKED"),
            "{record}: {audit}"
        );
        assert!(audit.contains(expected_row), "{record}: {audit}");
        assert!(
            !drops(&workspace).contains("malformed"),
            "文法外の綴りは形の違いではない: {}",
            drops(&workspace)
        );
    }
}

#[test]
fn an_orphaned_dispatch_record_is_ignored_and_cleaned_up() {
    let workspace = Workspace::new();
    write_dispatch(&workspace, &[]);
    let path = workspace.record().join(".aidlc-reviewer-dispatch.json");
    // 鮮度の窓 (6 時間) より古い記録は、判定に使わず掃除する。
    fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(7 * 60 * 60))
        .unwrap();
    let output = workspace.hook(
        "reviewer-scope",
        &reviewer_read(
            &workspace,
            "construction/u2-y/functional-design/entities.md",
        ),
        &[],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(!path.exists(), "古い記録は掃除される");
    assert!(
        drops(&workspace).contains("orphaned reviewer dispatch record"),
        "{}",
        drops(&workspace)
    );
    assert!(!workspace.audit().contains("REVIEWER_SCOPE_BLOCKED"));
}

#[test]
fn the_off_switch_stops_enforcement_as_well_as_observation() {
    let workspace = Workspace::new();
    write_dispatch(&workspace, &[]);
    let output = workspace.hook(
        "reviewer-scope",
        &reviewer_read(
            &workspace,
            "construction/u2-y/functional-design/entities.md",
        ),
        &[("AIDLC_DISABLE_REVIEWER_SCOPE_HOOK", "1")],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(!workspace.audit().contains("REVIEWER_SCOPE_BLOCKED"));
}

#[test]
fn a_call_without_a_working_directory_resolves_relative_roots_against_the_project_dir() {
    let workspace = Workspace::new();
    write_dispatch(&workspace, &[]);
    // upstream は封筒に `cwd` が無ければ project dir を基点にする
    // (`cwd: … ? cwdField : projectDir`)。`aidlc` は project dir から見て construction/ の
    // 上にある探索根なので、綴りに construction が出なくても全兄弟を掃く。固定 2.7.1 の
    // 実走行 (hook-differential c22) はこの入力を exit 2 / Target `aidlc` で拒否した。
    let output = workspace.hook(
        "reviewer-scope",
        r#"{"tool_name":"Grep","agent_type":"aidlc-architecture-reviewer-agent","tool_input":{"pattern":"x","path":"aidlc"}}"#,
        &[],
    );
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).starts_with("This review cannot open \"aidlc\""),
        "{output:?}"
    );
    assert!(
        workspace.audit().contains("**Target**: aidlc"),
        "{}",
        workspace.audit()
    );
}

#[test]
fn the_freeze_releases_a_zero_unit_stage_level_write_but_holds_the_unit_write() {
    let workspace = Workspace::new();
    let dir = workspace.record().join("construction/code-generation");
    fs::create_dir_all(&dir).unwrap();
    for (name, body) in [
        ("code-generation-plan.md", "# Plan\n"),
        ("unit-test-instructions.md", "# Test\n"),
        ("code-summary.md", "# Code\n"),
        ("traceability.json", "{}\n"),
    ] {
        fs::write(dir.join(name), body).unwrap();
    }
    let reviewer = "aidlc-architecture-reviewer-agent";
    let request = workspace.run(
        "aidlc-log",
        &[
            "review",
            "--stage",
            "code-generation",
            "--reviewer",
            reviewer,
            "--iteration",
            "1",
        ],
        &[],
    );
    assert!(request.status.success(), "{request:?}");
    // 判定が返る前は凍結しない。
    let before = workspace.hook(
        "review-freeze",
        &workspace.write_input("construction/code-generation/code-generation-plan.md"),
        &[],
    );
    assert_eq!(before.status.code(), Some(0), "{before:?}");
    fs::write(
        dir.join("code-generation-plan.md"),
        format!("# Plan\n\n## Review\n\n**Reviewer:** {reviewer}\n**Verdict:** READY\n**Iteration:** 1\n"),
    )
    .unwrap();
    let completed = workspace.run(
        "aidlc-log",
        &[
            "review",
            "--stage",
            "code-generation",
            "--reviewer",
            reviewer,
            "--iteration",
            "1",
            "--verdict",
            "READY",
        ],
        &[],
    );
    assert!(completed.status.success(), "{completed:?}");
    // ゼロ Unit の実行はステージ直下へ書く。per-unit ステージの受領証は Unit ごとなので、
    // ステージ水準の書込みは凍結しない (本家 `judgeFreeze` の `unit-of-work` 枝と同じ)。
    for relative in [
        "construction/code-generation/traceability.json",
        "construction/code-generation/code-generation-plan.md",
    ] {
        let allowed = workspace.hook("review-freeze", &workspace.write_input(relative), &[]);
        assert_eq!(allowed.status.code(), Some(0), "{relative}: {allowed:?}");
        assert!(
            !workspace.audit().contains("REVIEW_FREEZE_BLOCKED"),
            "通した書込みは拒否行を残さない: {relative}"
        );
    }
    // Unit 配下の書込みは Unit を名乗って拒否される。
    let unit = workspace.hook(
        "review-freeze",
        &workspace.write_input("construction/u1-x/code-generation/code-generation-plan.md"),
        &[],
    );
    assert_eq!(unit.status.code(), Some(2), "{unit:?}");
    let stderr = String::from_utf8_lossy(&unit.stderr);
    assert!(stderr.contains("stage \"code-generation\""), "{stderr}");
    assert!(stderr.contains("unit \"u1-x\""), "{stderr}");
    let audit = workspace.audit();
    assert!(audit.contains("**Stage**: code-generation"), "{audit}");
    assert!(audit.contains("**Unit**: u1-x"), "{audit}");
}

#[test]
fn a_shell_write_to_a_declared_artifact_is_refused_and_recorded() {
    let workspace = Workspace::new();
    workspace.terminal_receipt();
    let path = workspace.path_text("inception/requirements-analysis/requirements.md");
    let output = workspace.hook(
        "review-freeze",
        &shell_input(&format!("printf x >> {path}"), None),
        &[],
    );
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("review-freeze: "),
        "拒否理由は逐語で始まる: {stderr}"
    );
    assert!(
        stderr.contains("stage \"requirements-analysis\""),
        "{stderr}"
    );
    assert!(output.stdout.is_empty(), "拒否は stdout を汚さない");
    let audit = workspace.audit();
    assert!(
        audit.contains("**Event**: REVIEW_FREEZE_BLOCKED"),
        "{audit}"
    );
    assert!(audit.contains("**Tool**: Bash"), "{audit}");
    assert!(
        audit.contains("inception/requirements-analysis/requirements.md"),
        "{audit}"
    );
    // 解析が入ったので「書込み先を検査せずに通した」という目印はもう残らない。
    let health = workspace.record().join(".aidlc-hooks-health");
    assert!(
        !health
            .join("review-freeze.uninspected-shell.last")
            .is_file(),
        "未検査の目印は不要になる"
    );
    let drops = fs::read_to_string(health.join("review-freeze.drops")).unwrap_or_default();
    assert!(
        !drops.contains("without inspecting its write targets"),
        "{drops}"
    );
}

#[test]
fn every_shell_form_the_matcher_reads_reaches_the_freeze() {
    let workspace = Workspace::new();
    workspace.terminal_receipt();
    let path = workspace.path_text("inception/requirements-analysis/requirements.md");
    let stage_dir = workspace.path_text("inception/requirements-analysis");
    for (command, cwd) in [
        (format!("rm -f {path}"), None),
        (format!("sed -i s/a/b/ {path}"), None),
        (format!("tee {path}"), None),
        (format!("cp /tmp/other.md {path}"), None),
        (format!("mv /tmp/other.md {path}"), None),
        (format!("touch {path}"), None),
        (format!("printf x 2> {path}"), None),
        (format!("sudo rm -f {path}"), None),
        (format!("echo ok && rm -f {path}"), None),
        (
            "rm -f requirements.md".to_string(),
            Some(stage_dir.as_str()),
        ),
    ] {
        let output = workspace.hook("review-freeze", &shell_input(&command, cwd), &[]);
        assert_eq!(output.status.code(), Some(2), "{command}: {output:?}");
    }
}

#[test]
fn a_read_only_or_unrelated_shell_call_passes_untouched() {
    let workspace = Workspace::new();
    workspace.terminal_receipt();
    let path = workspace.path_text("inception/requirements-analysis/requirements.md");
    let diary = workspace.path_text("inception/requirements-analysis/memory.md");
    for command in [
        format!("cat {path}"),
        format!("grep -n x {path}"),
        format!("rm -f {path}.bak"),
        format!("printf x >> {diary}"),
        // `2>&1` は記述子の複製であり、`2` は `rm` の被演算子ではない。
        "rm -f /tmp/other.md 2>&1".to_string(),
        "mkdir -p /tmp/other".to_string(),
    ] {
        let output = workspace.hook("review-freeze", &shell_input(&command, None), &[]);
        assert_eq!(output.status.code(), Some(0), "{command}: {output:?}");
    }
    assert!(!workspace.audit().contains("REVIEW_FREEZE_BLOCKED"));
}

#[test]
fn a_malformed_shell_envelope_is_allowed_without_a_record() {
    let workspace = Workspace::new();
    workspace.terminal_receipt();
    for input in [
        r#"{"tool_name":"Bash"}"#,
        r#"{"tool_name":"Bash","tool_input":{}}"#,
        r#"{"tool_name":"Bash","tool_input":{"command":42}}"#,
        r#"{"tool_name":"Bash","tool_input":{"command":""}}"#,
    ] {
        let output = workspace.hook("review-freeze", input, &[]);
        assert_eq!(output.status.code(), Some(0), "{input}: {output:?}");
    }
    assert!(!workspace.audit().contains("REVIEW_FREEZE_BLOCKED"));
}

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;

#[path = "../../../../tests/support/tool_link.rs"]
mod tool_link;
