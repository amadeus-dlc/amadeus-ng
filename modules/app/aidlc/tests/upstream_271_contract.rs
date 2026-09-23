//! 固定した本家2.7.1の配布定義とネイティブ実行入口の契約。
#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

struct Workspace {
    root: tempfile::TempDir,
}

/// 本家2.7.1 `emitError`（aidlc-lib.ts:22208-22245）は拒否文言を `JSON.stringify({ error: msg })` の
/// 1 行として stderr へ出し exit 1 する。`aidlc-log.ts` の `error()`（:2313-2317）はこれ以外の
/// 出力経路を持たないので、平文ではなくこの形が契約である。
#[expect(
    clippy::disallowed_methods,
    reason = "型付き契約の変換ではなく、本家のワイヤ形式そのものを期待値として組む (BR1.7 の射程外)"
)]
fn upstream_error(message: &str) -> String {
    format!("{}\n", serde_json::json!({ "error": message }))
}

#[test]
fn stop_preserves_the_selected_response_when_the_counter_cannot_be_published() {
    let workspace = Workspace::brownfield();
    assert!(workspace.create().status.success());
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let marker = record.join(".aidlc-stop-hook/block-count.json");
    fs::create_dir_all(&marker).unwrap();
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/stop-publication.json"
    ))
    .unwrap();
    for invocation in 1..=2 {
        let id = format!("counter-directory/{invocation}");
        let expected = corpus
            .get("observations")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case.get("id").and_then(serde_json::Value::as_str) == Some(&id))
            .unwrap()
            .get("output")
            .unwrap();
        let actual = workspace.stop(r#"{"stop_hook_active":false}"#, &[]);
        assert_eq!(actual.status.code(), Some(0));
        let source_output: serde_json::Value =
            serde_json::from_str(expected.get("stdout").unwrap().as_str().unwrap()).unwrap();
        let actual_output: serde_json::Value = serde_json::from_slice(&actual.stdout).unwrap();
        assert_eq!(
            actual_output.get("decision"),
            source_output.get("decision"),
            "{id}"
        );
        let rendering: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/golden/selfhost-stage1/stop-values.json"
        ))
        .unwrap();
        let rendered = rendering
            .get("observations")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .find(|case| {
                case.get("id").and_then(serde_json::Value::as_str)
                    == Some("run-reverse-engineering")
            })
            .unwrap();
        assert_eq!(
            String::from_utf8(actual.stdout).unwrap(),
            rendered.get("stdout").unwrap().as_str().unwrap(),
            "{id}: 同じrun-stage入力の本家描画全文"
        );
        assert_eq!(
            actual.stderr,
            expected.get("stderr").unwrap().as_str().unwrap().as_bytes(),
            "{id}"
        );
        assert!(marker.is_dir());
    }
}

#[test]
fn stop_blocks_pending_work_once_then_releases_without_workflow_progress() {
    use std::io::Write as _;
    let workspace = Workspace::brownfield();
    let created = workspace.create();
    assert!(created.status.success(), "{created:?}");
    let state_before = workspace.state();
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let mut outputs = Vec::new();
    for _ in 0..2 {
        let mut child = Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .args(["hook", "continue-workflow"])
            .current_dir(workspace.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", workspace.path())
            .env("PATH", "/usr/bin:/bin")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"{\"stop_hook_active\":false}")
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert!(output.stderr.is_empty(), "{output:?}");
        outputs.push(output.stdout);
    }
    let first: serde_json::Value = serde_json::from_slice(outputs.first().unwrap()).unwrap();
    assert_eq!(
        first.get("decision").and_then(serde_json::Value::as_str),
        Some("block")
    );
    let reason = first
        .get("reason")
        .and_then(serde_json::Value::as_str)
        .unwrap();
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/stop-values.json"
    ))
    .unwrap();
    let expected = reference
        .get("observations")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|case| {
            case.get("id").and_then(serde_json::Value::as_str) == Some("run-reverse-engineering")
        })
        .unwrap();
    assert_eq!(reason, expected.get("reason").unwrap().as_str().unwrap());
    assert_eq!(
        outputs.first().unwrap(),
        expected.get("stdout").unwrap().as_str().unwrap().as_bytes()
    );
    assert!(outputs.get(1).unwrap().is_empty());
    let counter: serde_json::Value = serde_json::from_slice(
        &fs::read(record.join(".aidlc-stop-hook/block-count.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        counter.get("count").and_then(serde_json::Value::as_u64),
        Some(2)
    );
    assert_eq!(workspace.state(), state_before);
}
impl Workspace {
    fn stop(&self, input: &str, environment: &[(&str, &str)]) -> Output {
        use std::io::Write as _;
        let mut command = Command::new(env!("CARGO_BIN_EXE_aidlc"));
        command
            .args(["hook", "continue-workflow"])
            .current_dir(self.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.path())
            .env("PATH", "/usr/bin:/bin")
            .envs(environment.iter().copied())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        let mut child = command.spawn().unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    }
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("workspace");
        fs::create_dir(&project).unwrap();
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let data = project.join(".claude/tools/data");
        fs::create_dir_all(&data).unwrap();
        for name in ["stage-graph.json", "scope-grid.json", "harness.json"] {
            fs::copy(
                repo.join("tests/golden/upstream-a277af21/data").join(name),
                data.join(name),
            )
            .unwrap();
        }
        fs::create_dir_all(project.join(".claude/scopes")).unwrap();
        fs::copy(
            repo.join(".claude/scopes/aidlc-bugfix.md"),
            project.join(".claude/scopes/aidlc-bugfix.md"),
        )
        .unwrap();
        fs::create_dir_all(project.join("aidlc/spaces/default/intents")).unwrap();
        fs::create_dir_all(project.join("bin")).unwrap();
        // `bin/` はワークスペース内にあり Source Baseline の走査対象になる。リンクだと採取が
        // `unbindable` になり床の sha が立たないので、この fixture だけは実体を置く。
        fs::copy(
            env!("CARGO_BIN_EXE_aidlc"),
            project.join("bin/aidlc-utility"),
        )
        .unwrap();
        Self { root }
    }
    fn path(&self) -> PathBuf {
        self.root.path().join("workspace")
    }

    /// ソースを 1 つ置いて Brownfield にする。本家 `intent-create` は Greenfield と走査した
    /// ワークスペースで reverse-engineering を SKIP する (`aidlc-utility.ts:5895-5904`) ので、
    /// 最初の run-stage が reverse-engineering であることに依る検査はこれを先に呼ぶ。
    fn brownfield() -> Self {
        let workspace = Self::new();
        fs::create_dir_all(workspace.path().join("src")).unwrap();
        fs::write(workspace.path().join("src/lib.rs"), "pub fn smoke() {}\n").unwrap();
        workspace
    }

    fn create(&self) -> Output {
        let mut command = Command::new(self.path().join("bin/aidlc-utility"));
        command
            .args([
                "intent-create",
                "--scope",
                "bugfix",
                "--label",
                "smoke",
                "--arguments",
                "Fix one small defect",
            ])
            .current_dir(self.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.path())
            .env("PATH", "/usr/bin:/bin");
        // 並列テストのforkがコピー中の書込fdを継承すると、close-on-execまでETXTBSYに
        // なることがある。起動前のこの失敗だけを待ち、実行後の終了結果はそのまま返す。
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        loop {
            match command.output() {
                Err(error)
                    if error.kind() == std::io::ErrorKind::ExecutableFileBusy
                        && std::time::Instant::now() < deadline =>
                {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                result => return result.unwrap(),
            }
        }
    }
    fn state(&self) -> String {
        fs::read_to_string(self.record_dir().join("aidlc-state.md")).unwrap()
    }
    /// requirements-analysis の `produces`（`requirements` / `requirements-analysis-questions`）を
    /// 記録へ書く。`reviewed` なら要求時のバイトの末尾へ `## Review` 付録だけを加える
    /// （ゴールデン `stage1/cases.json` の `review/request` と `review/completed` と同じ形）。
    fn write_requirements_documents(&self, reviewed: bool) {
        let directory = self.record_dir().join("inception/requirements-analysis");
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
    /// 有効な intent の記録ディレクトリ。
    fn record_dir(&self) -> PathBuf {
        let intents = self.path().join("aidlc/spaces/default/intents");
        let cursor = fs::read_to_string(intents.join("active-intent")).unwrap();
        intents.join(cursor.trim())
    }
    /// 本家2.7.1 `checkPipelineLinkEvidence`（aidlc-orchestrate.ts:7198-7227）は `mode: pipeline` の
    /// reverse-engineering を `[?]` へ進める前に lead/support 全リンクの当該試行の受領証を要求し、
    /// 欠ければ `Cannot present ... for approval` を返す。developer リンクは handoff 成果物
    /// `<record>/inception/reverse-engineering/developer-scan.md` を要する（aidlc-log.ts:895-979）。
    fn complete_reverse_engineering_pipeline(&self) {
        let handoff = self
            .record_dir()
            .join("inception/reverse-engineering/developer-scan.md");
        fs::create_dir_all(handoff.parent().unwrap()).unwrap();
        fs::write(&handoff, "## Developer Code Scan Results\n").unwrap();
        let artifact = handoff
            .strip_prefix(self.path())
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        for (agent, artifact) in [
            ("aidlc-developer-agent", Some(artifact.as_str())),
            ("aidlc-architect-agent", None),
        ] {
            let mut args = vec!["link", "--stage", "reverse-engineering", "--link", agent];
            if let Some(artifact) = artifact {
                args.extend(["--artifact", artifact]);
            }
            let linked = self.log(&args);
            assert!(linked.status.success(), "{linked:?}");
            assert!(
                String::from_utf8_lossy(&linked.stdout)
                    .contains("\"emitted\":\"PIPELINE_LINK_COMPLETED\""),
                "{linked:?}"
            );
        }
    }
    fn log(&self, args: &[&str]) -> Output {
        let binary = self.path().join("bin/aidlc-log");
        if !binary.exists() {
            fs::copy(env!("CARGO_BIN_EXE_aidlc"), &binary).unwrap();
        }
        let home = self.path().join("aidlc/.capture-home");
        fs::create_dir_all(&home).unwrap();
        Command::new(binary)
            .args(args)
            .current_dir(self.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", home)
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap()
    }
    fn human_prompt(&self, input: &str, unattended: bool) -> Output {
        self.human_bytes(input.as_bytes(), unattended)
    }
    fn human_bytes(&self, input: &[u8], unattended: bool) -> Output {
        use std::io::Write as _;
        let mut command = Command::new(env!("CARGO_BIN_EXE_aidlc"));
        command
            .args(["hook", "record-human-turn"])
            .current_dir(self.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.path())
            .env("PATH", "/usr/bin:/bin")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        if unattended {
            command.env("AIDLC_UNATTENDED", "1");
        }
        let mut child = command.spawn().unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        child.wait_with_output().unwrap()
    }
}

#[cfg(target_os = "linux")]
#[test]
fn intent_creation_waits_for_the_copied_executables_writer_to_close() {
    let workspace = Workspace::new();
    let binary = workspace.path().join("bin/aidlc-utility");
    let writer = fs::OpenOptions::new().write(true).open(&binary).unwrap();
    assert_eq!(
        Command::new(&binary)
            .arg("--help")
            .output()
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::ExecutableFileBusy,
    );
    let release = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        drop(writer);
    });
    let result = workspace.create();
    release.join().unwrap();
    assert!(result.status.success(), "{result:?}");
}

#[test]
fn stop_recovers_failed_publication_before_observing_the_next_counter() {
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    assert!(!workspace.stop("{}", &[]).stdout.is_empty());
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let marker = record.join(".aidlc-stop-hook/block-count.json");
    fs::remove_file(&marker).unwrap();
    fs::create_dir(&marker).unwrap();
    let root = workspace.path().join("aidlc");
    let store = core_command_domain::workspace::StorePath::for_runtime(&root);
    let db = rusqlite::Connection::open(store.as_path()).unwrap();
    db.execute_batch("CREATE TRIGGER fail_counter_confirmation BEFORE INSERT ON journal WHEN NEW.manifest='workflow-continuation-event/1' AND json_extract(CAST(NEW.payload AS TEXT),'$.publication') IS NOT NULL BEGIN SELECT RAISE(ABORT,'injected confirmation failure'); END;").unwrap();
    let stopped = workspace.stop("{}", &[]);
    assert!(stopped.stdout.is_empty() && stopped.stderr.is_empty());
    let pending: (String, bool) = db
        .query_row(
            "SELECT id,counter_published FROM read_continuation_result WHERE settled=0",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert!(!pending.1);
    assert!(marker.is_dir());
    db.execute_batch("DROP TRIGGER fail_counter_confirmation")
        .unwrap();
    fs::remove_dir(&marker).unwrap();
    let resumed = workspace.stop("{}", &[]);
    assert!(
        !resumed.stdout.is_empty() && resumed.stderr.is_empty(),
        "失敗した旧公開を再試行してcountを前進させない: {resumed:?}"
    );
    let counter: serde_json::Value = serde_json::from_slice(&fs::read(&marker).unwrap()).unwrap();
    assert_eq!(
        counter.get("count").and_then(serde_json::Value::as_u64),
        Some(1)
    );
    let prior: (bool, bool) = db
        .query_row(
            "SELECT counter_published,settled FROM read_continuation_result WHERE id=?1",
            [&pending.0],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(prior, (false, true));
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM read_continuation_result WHERE settled=0",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}

#[test]
fn stop_observes_counter_replacement_and_recovers_after_the_directory_is_removed() {
    let workspace = Workspace::new();
    fs::create_dir_all(workspace.path().join("src")).unwrap();
    fs::write(workspace.path().join("src/lib.rs"), "pub fn smoke() {}\n").unwrap();
    assert!(workspace.create().status.success());
    let first = workspace.stop("{}", &[]);
    assert!(!first.stdout.is_empty() && first.stderr.is_empty());
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let marker = record.join(".aidlc-stop-hook/block-count.json");
    fs::remove_file(&marker).unwrap();
    fs::create_dir(&marker).unwrap();
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/stop-publication.json"
    ))
    .unwrap();
    for (index, scenario) in ["directory-1", "directory-2", "recovered"]
        .into_iter()
        .enumerate()
    {
        if index == 2 {
            fs::remove_dir(&marker).unwrap();
        }
        let expected = corpus
            .get("observations")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .find(|case| {
                case.get("id").and_then(serde_json::Value::as_str)
                    == Some(&format!("brownfield-counter/{scenario}"))
            })
            .unwrap()
            .get("output")
            .unwrap();
        let source: serde_json::Value =
            serde_json::from_str(expected.get("stdout").unwrap().as_str().unwrap()).unwrap();
        let actual = workspace.stop("{}", &[]);
        assert_eq!(
            actual.status.code(),
            expected
                .get("exit_code")
                .and_then(serde_json::Value::as_i64)
                .map(|code| i32::try_from(code).unwrap())
        );
        assert_eq!(
            actual.stderr,
            expected.get("stderr").unwrap().as_str().unwrap().as_bytes()
        );
        let output: serde_json::Value = serde_json::from_slice(&actual.stdout).unwrap();
        assert_eq!(output.get("decision"), source.get("decision"));
        assert_eq!(
            actual.stdout, first.stdout,
            "公開障害で選んだ継続指示を変えない: {scenario}"
        );
        if index < 2 {
            assert!(marker.is_dir());
        }
    }
    let restored: serde_json::Value = serde_json::from_slice(&fs::read(&marker).unwrap()).unwrap();
    assert_eq!(
        restored.get("count").and_then(serde_json::Value::as_u64),
        Some(1)
    );
}

#[test]
fn stop_waits_only_for_the_active_stages_logged_decision_until_answered() {
    let workspace = Workspace::brownfield();
    assert!(workspace.create().status.success());
    assert!(
        workspace
            .log(&[
                "decision",
                "--stage",
                "requirements-analysis",
                "--decision",
                "別段階の質問",
                "--options",
                "A,B"
            ])
            .status
            .success()
    );
    assert!(!workspace.stop("{}", &[]).stdout.is_empty());
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let marker = record.join(".aidlc-stop-hook/block-count.json");
    let before = fs::read(&marker).unwrap();
    assert!(
        workspace
            .log(&[
                "decision",
                "--stage",
                "reverse-engineering",
                "--decision",
                "現段階の質問",
                "--options",
                "A,B"
            ])
            .status
            .success()
    );
    let waiting = workspace.stop("{}", &[]);
    assert!(waiting.status.success() && waiting.stdout.is_empty() && waiting.stderr.is_empty());
    assert_eq!(fs::read(&marker).unwrap(), before);
    assert!(workspace.human_prompt(r#"{"hook_event_name":"UserPromptSubmit","session_id":"stop-session","prompt":"A"}"#, false).status.success());
    assert!(
        workspace
            .log(&["answer", "--stage", "reverse-engineering", "--details", "A"])
            .status
            .success()
    );
    let resumed = workspace.stop("{}", &[]);
    assert!(resumed.status.success() && resumed.stdout.is_empty() && resumed.stderr.is_empty());
    let counter: serde_json::Value = serde_json::from_slice(&fs::read(&marker).unwrap()).unwrap();
    assert_eq!(
        counter.get("count").and_then(serde_json::Value::as_u64),
        Some(2)
    );
}

#[test]
fn concurrent_stop_requests_publish_one_serial_counter_sequence() {
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    let outputs = std::thread::scope(|scope| {
        let first = scope.spawn(|| workspace.stop("{}", &[]));
        let second = scope.spawn(|| workspace.stop("{}", &[]));
        vec![first.join().unwrap(), second.join().unwrap()]
    });
    assert!(
        outputs
            .iter()
            .all(|output| output.status.success() && output.stderr.is_empty())
    );
    assert_eq!(
        outputs
            .iter()
            .filter(|output| !output.stdout.is_empty())
            .count(),
        1
    );
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let counter: serde_json::Value = serde_json::from_slice(
        &fs::read(record.join(".aidlc-stop-hook/block-count.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        counter.get("count").and_then(serde_json::Value::as_u64),
        Some(2)
    );
}

#[test]
fn stop_limit_honors_the_numeric_prefix_and_reentrant_initial_count() {
    for limit in ["3", "3trailing"] {
        let workspace = Workspace::new();
        assert!(workspace.create().status.success());
        let first = workspace.stop(
            r#"{"stop_hook_active":true}"#,
            &[("CLAUDE_CODE_STOP_HOOK_BLOCK_CAP", limit)],
        );
        assert_eq!(first.status.code(), Some(0), "{first:?}");
        assert!(
            !first.stdout.is_empty(),
            "limit={limit}: 再入2回目は上限3の手前なので差し止める"
        );
        let second = workspace.stop(
            r#"{"stop_hook_active":true}"#,
            &[("CLAUDE_CODE_STOP_HOOK_BLOCK_CAP", limit)],
        );
        assert_eq!(second.status.code(), Some(0), "{second:?}");
        assert!(second.stdout.is_empty(), "limit={limit}: 3回目で解除する");
    }
}

#[test]
fn stop_allows_an_open_human_gate_without_starting_a_no_progress_streak() {
    let workspace = Workspace::brownfield();
    assert!(workspace.create().status.success());
    workspace.complete_reverse_engineering_pipeline();
    let opened = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .args(["report", "--result", "awaiting-approval"])
        .current_dir(workspace.path())
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", workspace.path())
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    assert!(opened.status.success(), "{opened:?}");
    let before = workspace.state();
    assert!(
        before.contains("- [?] reverse-engineering"),
        "承認ゲートが開いていなければ human-wait carve-out は成立しない: {opened:?}"
    );
    let output = workspace.stop("{}", &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(
        output.stdout.is_empty(),
        "人間の承認待ちに継続指示を出さない: {output:?}"
    );
    assert!(output.stderr.is_empty());
    assert_eq!(workspace.state(), before);
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    assert!(!record.join(".aidlc-stop-hook/block-count.json").exists());
}

#[test]
fn stop_waits_for_an_unanswered_question_only_in_the_active_stage_directory() {
    let workspace = Workspace::brownfield();
    assert!(workspace.create().status.success());
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let questions = record.join("inception/reverse-engineering/clarification-questions.md");
    fs::create_dir_all(questions.parent().unwrap()).unwrap();
    fs::write(&questions, "# Questions\n[Answer]: ___\n").unwrap();
    let waiting = workspace.stop("{}", &[]);
    assert_eq!(waiting.status.code(), Some(0));
    assert!(
        waiting.stdout.is_empty(),
        "未回答質問の待機を妨げない: {waiting:?}"
    );
    assert!(!record.join(".aidlc-stop-hook/block-count.json").exists());
    fs::write(&questions, "# Questions\n[Answer]: Answered\n").unwrap();
    let sibling = record.join("inception/requirements-analysis/stale-questions.md");
    fs::create_dir_all(sibling.parent().unwrap()).unwrap();
    fs::write(sibling, "[Answer]:\n").unwrap();
    let resumed = workspace.stop("{}", &[]);
    assert!(
        !resumed.stdout.is_empty(),
        "他ステージの未回答は現ステージを止めない: {resumed:?}"
    );
}

#[test]
fn a_question_wait_does_not_create_prior_counter_evidence_for_a_reentrant_stop() {
    let workspace = Workspace::brownfield();
    assert!(workspace.create().status.success());
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let question = record.join("inception/reverse-engineering/clarification-questions.md");
    fs::create_dir_all(question.parent().unwrap()).unwrap();
    fs::write(&question, "[Answer]:\n").unwrap();
    assert!(workspace.stop("{}", &[]).stdout.is_empty());
    fs::write(question, "[Answer]: Yes\n").unwrap();
    let resumed = workspace.stop(r#"{"stop_hook_active":true}"#, &[]);
    assert!(
        resumed.stdout.is_empty(),
        "counter未観測の再入は2から開始する: {resumed:?}"
    );
    let counter: serde_json::Value = serde_json::from_slice(
        &fs::read(record.join(".aidlc-stop-hook/block-count.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        counter.get("count").and_then(serde_json::Value::as_u64),
        Some(2)
    );
}

#[test]
fn stop_uses_the_payload_sessions_binding_without_switching_the_shared_cursor() {
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let first = fs::read_to_string(intents.join("active-intent")).unwrap();
    assert!(
        Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .arg("park")
            .current_dir(workspace.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", workspace.path())
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(workspace.create().status.success());
    let second = fs::read_to_string(intents.join("active-intent")).unwrap();
    assert_ne!(first, second);
    let sessions = workspace.path().join("aidlc/.aidlc-sessions");
    fs::create_dir_all(&sessions).unwrap();
    fs::write(
        sessions.join("first-session.binding.json"),
        format!(
            r#"{{"space":"default","intent":"{}","boundAt":"2026-09-09T00:00:00Z"}}"#,
            first.trim()
        ),
    )
    .unwrap();
    let output = workspace.stop(r#"{"session_id":"first-session"}"#, &[]);
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "別sessionの作業へ継続指示を出さない: {output:?}"
    );
    assert_eq!(
        fs::read_to_string(intents.join("active-intent")).unwrap(),
        second
    );
    assert_eq!(
        fs::read_to_string(
            intents
                .join(first.trim())
                .join(".aidlc-stop-hook/block-count.json")
        )
        .unwrap(),
        r#"{"signature":"","count":0}"#
    );
    assert!(
        !intents
            .join(second.trim())
            .join(".aidlc-stop-hook/block-count.json")
            .exists()
    );
}

#[test]
fn stop_allows_conversation_but_not_engine_work_recorded_after_the_human_prompt() {
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    let transcript = workspace.path().join("conversation.jsonl");
    let human = "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"Explain this decision\"}}\n";
    fs::write(&transcript, format!("{human}{{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":[{{\"type\":\"text\",\"text\":\"The explanation\"}}]}}}}\n")).unwrap();
    let input = format!(r#"{{"transcript_path":"{}"}}"#, transcript.display());
    let conversation = workspace.stop(&input, &[]);
    assert!(
        conversation.stdout.is_empty() && conversation.stderr.is_empty(),
        "会話への回答を差し止めない: {conversation:?}"
    );
    fs::write(&transcript, format!("{human}{{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":[{{\"type\":\"tool_use\",\"name\":\"Bash\",\"input\":{{\"command\":\"aidlc next\"}}}}]}}}}\n")).unwrap();
    let engaged = workspace.stop(&input, &[]);
    assert!(
        !engaged.stdout.is_empty(),
        "engine呼出し後は通常の停止制御へ戻す: {engaged:?}"
    );
}

#[test]
fn stop_preserves_a_state_bound_shared_resume_prompt() {
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    assert!(
        Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .arg("next")
            .current_dir(workspace.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", workspace.path())
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap()
            .status
            .success()
    );
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let marker_path = record.join(".aidlc-active-directive.json");
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
    let bytes = core_infrastructure::canon_json::serialize(
        &core_infrastructure::canon_json::to_value(&marker).unwrap(),
        core_infrastructure::canon_json::SerializationProfile::ContractPretty,
    );
    fs::write(&marker_path, &bytes).unwrap();
    let stopped = workspace.stop("{}", &[]);
    assert!(
        stopped.stdout.is_empty() && stopped.stderr.is_empty(),
        "再開方法の人間回答を待つ: {stopped:?}"
    );
    assert_eq!(fs::read_to_string(&marker_path).unwrap(), bytes);
    assert!(!record.join(".aidlc-stop-hook/block-count.json").exists());
    marker.as_object_mut().unwrap().insert(
        "state_sha256".into(),
        serde_json::Value::String("0".repeat(64)),
    );
    fs::write(
        &marker_path,
        core_infrastructure::canon_json::serialize(
            &core_infrastructure::canon_json::to_value(&marker).unwrap(),
            core_infrastructure::canon_json::SerializationProfile::ContractPretty,
        ),
    )
    .unwrap();
    assert!(
        !workspace.stop("{}", &[]).stdout.is_empty(),
        "古いstate hashでは待機を認めない"
    );
    let fields = marker.as_object_mut().unwrap();
    fields.insert(
        "state_sha256".into(),
        serde_json::Value::String(core_infrastructure::hash::sha256_hex(
            workspace.state().as_bytes(),
        )),
    );
    fields.insert(
        "owner_session".into(),
        serde_json::Value::String("foreign-session".into()),
    );
    fs::write(
        &marker_path,
        core_infrastructure::canon_json::serialize(
            &core_infrastructure::canon_json::to_value(&marker).unwrap(),
            core_infrastructure::canon_json::SerializationProfile::ContractPretty,
        ),
    )
    .unwrap();
    assert!(workspace.stop("{}", &[]).stdout.is_empty());
    let counter: serde_json::Value = serde_json::from_slice(
        &fs::read(record.join(".aidlc-stop-hook/block-count.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        counter.get("count").and_then(serde_json::Value::as_u64),
        Some(2),
        "sessionlessでないmarkerはcounterを止めない"
    );
}

#[tokio::test]
async fn stop_request_results_recover_by_their_original_ids_after_projection_failure() {
    use core_command_domain::orchestration::{
        ContinuationAttemptId, ContinuationObservations, ContinuationQuestions,
        ContinuationRequest, ContinuationSignature, WorkflowContinuationId,
    };
    use core_command_domain::workspace::{SpaceName, StorePath};
    use core_command_interface_adapter::orchestration::{
        IntentExecutionRepositoryImpl, WorkflowContinuationRepositoryImpl,
    };
    use core_command_use_case::orchestration::RecordContinuationUseCase;
    use core_query_interface_adapter::ReadModelDaos;
    use core_query_use_case::orchestration::ContinuationResultUseCase;
    use core_read_model_updater::orchestration::WorkflowContinuationReadModelUpdater;
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    assert!(!workspace.stop("{}", &[]).stdout.is_empty());
    let root = workspace.path().join("aidlc");
    let intents = root.join("spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let execution = aidlc::execution_cursor::ExecutionCursor::read(&record)
        .unwrap()
        .unwrap();
    let id = WorkflowContinuationId::for_execution(execution.execution_id());
    let store = StorePath::for_runtime(&root);
    let source = StorePath::for_space(&root, &SpaceName::parse("default").unwrap());
    let request_id = ContinuationAttemptId::generate();
    let signature = ContinuationSignature::parse(&format!(
        "reverse-engineering::{}::{}",
        "a".repeat(64),
        "b".repeat(64)
    ))
    .unwrap();
    let observations = ContinuationObservations::new(ContinuationQuestions::new(Vec::new()), false);
    RecordContinuationUseCase::new(
        WorkflowContinuationRepositoryImpl::open(&store).unwrap(),
        IntentExecutionRepositoryImpl::open(&source).unwrap(),
    )
    .execute(
        execution.execution_id(),
        ContinuationRequest::new(request_id.clone(), Some(signature.clone()), false, 4).unwrap(),
        Some("4"),
        &observations,
        chrono::Utc::now(),
    )
    .await
    .unwrap();
    let query = || {
        ContinuationResultUseCase::new(
            ReadModelDaos::open(store.as_path())
                .unwrap()
                .continuation_result(),
        )
    };
    assert!(query().execute(request_id.as_str()).unwrap().is_none());
    let connection = rusqlite::Connection::open(store.as_path()).unwrap();
    connection.execute_batch("CREATE TRIGGER fail_stop_result BEFORE INSERT ON read_continuation_result BEGIN SELECT RAISE(ABORT, 'injected stop projection failure'); END;").unwrap();
    assert!(
        WorkflowContinuationReadModelUpdater::open(store.as_path())
            .unwrap()
            .catch_up(&id, &record)
            .is_err()
    );
    assert!(
        query().execute(request_id.as_str()).unwrap().is_none(),
        "投影失敗を結果の成功へ丸めない"
    );
    connection
        .execute_batch("DROP TRIGGER fail_stop_result")
        .unwrap();
    WorkflowContinuationReadModelUpdater::open(store.as_path())
        .unwrap()
        .catch_up(&id, &record)
        .unwrap();
    let observed = query().execute(request_id.as_str()).unwrap().unwrap();
    core_command_use_case::orchestration::SettleContinuationPublicationUseCase::new(
        WorkflowContinuationRepositoryImpl::open(&store).unwrap(),
    )
    .execute(
        &id,
        &core_command_domain::orchestration::ContinuationPublicationObservation::new(
            request_id.clone(),
            observed.published().unwrap(),
        ),
        chrono::Utc::now(),
    )
    .await
    .unwrap();
    WorkflowContinuationReadModelUpdater::open(store.as_path())
        .unwrap()
        .catch_up(&id, &record)
        .unwrap();
    let recovered = query().execute(request_id.as_str()).unwrap().unwrap();
    assert!(recovered.blocked());
    assert_eq!(recovered.count(), 1);
    assert_eq!(recovered.limit(), 4);
    let later_id = ContinuationAttemptId::generate();
    RecordContinuationUseCase::new(
        WorkflowContinuationRepositoryImpl::open(&store).unwrap(),
        IntentExecutionRepositoryImpl::open(&source).unwrap(),
    )
    .execute(
        execution.execution_id(),
        ContinuationRequest::new(later_id.clone(), Some(signature), false, 4).unwrap(),
        Some("4"),
        &observations,
        chrono::Utc::now(),
    )
    .await
    .unwrap();
    WorkflowContinuationReadModelUpdater::open(store.as_path())
        .unwrap()
        .catch_up(&id, &record)
        .unwrap();
    assert_eq!(
        query().execute(request_id.as_str()).unwrap(),
        Some(recovered)
    );
    assert_eq!(
        query().execute(later_id.as_str()).unwrap().unwrap().count(),
        2
    );
    WorkflowContinuationReadModelUpdater::open(store.as_path())
        .unwrap()
        .catch_up(&id, &record)
        .unwrap();
    assert_eq!(
        query().execute(later_id.as_str()).unwrap().unwrap().count(),
        2,
        "反復投影でcountを増やさない"
    );
}

#[test]
fn stop_projection_rejects_a_first_event_that_claims_an_unobserved_count() {
    use core_command_domain::orchestration::WorkflowContinuationId;
    use core_read_model_updater::orchestration::WorkflowContinuationReadModelUpdater;
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    assert!(!workspace.stop("{}", &[]).stdout.is_empty());
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let cursor = aidlc::execution_cursor::ExecutionCursor::read(&record)
        .unwrap()
        .unwrap();
    let store = workspace.path().join("aidlc/.aidlc-runtime.sqlite");
    let marker = record.join(".aidlc-stop-hook/block-count.json");
    let before = fs::read(&marker).unwrap();
    let db = rusqlite::Connection::open(&store).unwrap();
    assert_eq!(db.execute("UPDATE journal SET payload=CAST(json_set(CAST(payload AS TEXT),'$.count',9,'$.blocked',json('false')) AS BLOB) WHERE manifest='workflow-continuation-event/1' AND seq_nr=1", []).unwrap(), 1);
    let result = WorkflowContinuationReadModelUpdater::open(&store)
        .unwrap()
        .catch_up(
            &WorkflowContinuationId::for_execution(cursor.execution_id()),
            &record,
        );
    assert!(result.is_err(), "誕生イベントのcount9を投影しない");
    assert_eq!(fs::read(marker).unwrap(), before);
}

#[test]
#[cfg(unix)]
fn stop_releases_within_the_engine_deadline_when_next_blocks_on_io() {
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    let rules = workspace.path().join("aidlc/spaces/default/memory/org.md");
    fs::create_dir_all(rules.parent().unwrap()).unwrap();
    assert!(
        Command::new("mkfifo")
            .arg(&rules)
            .status()
            .unwrap()
            .success()
    );
    let start = std::time::Instant::now();
    let stopped = workspace.stop("{}", &[]);
    let elapsed = start.elapsed();
    assert!(
        elapsed >= std::time::Duration::from_secs(9),
        "実際にnextが待機したことを確認する: {elapsed:?}"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(20),
        "Stopを無期限に待たせない: {elapsed:?}"
    );
    assert_eq!(stopped.status.code(), Some(0));
    assert!(
        stopped.stdout.is_empty() && stopped.stderr.is_empty(),
        "{stopped:?}"
    );
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    assert!(!record.join(".aidlc-stop-hook/block-count.json").exists());
    assert!(
        fs::read_to_string(record.join(".aidlc-hooks-health/continue-workflow.drops"))
            .unwrap()
            .contains("engine next returned no parseable directive; allowing stop")
    );
}

#[test]
fn a_stop_storage_failure_keeps_the_previous_counter_and_allows_the_turn_to_end() {
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    assert!(!workspace.stop("{}", &[]).stdout.is_empty());
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let marker = record.join(".aidlc-stop-hook/block-count.json");
    let before = fs::read(&marker).unwrap();
    let state = workspace.state();
    let db =
        rusqlite::Connection::open(workspace.path().join("aidlc/.aidlc-runtime.sqlite")).unwrap();
    db.execute_batch("CREATE TRIGGER fail_stop_write BEFORE INSERT ON journal WHEN NEW.manifest='workflow-continuation-event/1' BEGIN SELECT RAISE(ABORT,'injected stop storage failure'); END;").unwrap();
    let failed = workspace.stop("{}", &[]);
    assert_eq!(failed.status.code(), Some(0));
    assert!(
        failed.stdout.is_empty() && failed.stderr.is_empty(),
        "{failed:?}"
    );
    assert_eq!(fs::read(&marker).unwrap(), before);
    assert_eq!(workspace.state(), state);
    db.execute_batch("DROP TRIGGER fail_stop_write").unwrap();
    assert!(workspace.stop("{}", &[]).stdout.is_empty());
    let restored: serde_json::Value = serde_json::from_slice(&fs::read(marker).unwrap()).unwrap();
    assert_eq!(
        restored.get("count").and_then(serde_json::Value::as_u64),
        Some(2)
    );
}

#[test]
fn stop_allows_the_turn_when_resume_evidence_is_busy_without_running_next() {
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    assert!(
        Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .arg("next")
            .current_dir(workspace.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", workspace.path())
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap()
            .status
            .success()
    );
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(workspace.path().join("aidlc/.aidlc-runtime.lock"))
        .unwrap();
    let _held =
        core_infrastructure::ExclusiveFileLock::acquire(file, std::time::Duration::ZERO).unwrap();
    let started = std::time::Instant::now();
    let stopped = workspace.stop("{}", &[]);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(3),
        "本家の100回×10msの待ちを大幅に延長しない"
    );
    assert_eq!(stopped.status.code(), Some(0));
    assert!(stopped.stdout.is_empty() && stopped.stderr.is_empty());
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    assert!(!record.join(".aidlc-stop-hook/block-count.json").exists());
    assert!(fs::read_to_string(record.join(".aidlc-hooks-health/continue-workflow.drops")).unwrap().contains("active-directive evidence unavailable while reading shared resume wait: Active-directive coordination is busy; allowing stop"));
}

#[test]
fn stop_consumes_the_exact_new_intent_session_handoff_once() {
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let first = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let from = aidlc::execution_cursor::ExecutionCursor::read(&first)
        .unwrap()
        .unwrap();
    assert!(workspace.create().status.success());
    let current = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let to = aidlc::execution_cursor::ExecutionCursor::read(&current)
        .unwrap()
        .unwrap();
    let sessions = workspace.path().join("aidlc/.aidlc-sessions");
    fs::create_dir_all(&sessions).unwrap();
    fs::write(sessions.join("handoff-session"), to.intent_id().as_str()).unwrap();
    let handoff = sessions.join("handoff-session.handoff.json");
    let body = serde_json::Value::Object(serde_json::Map::from_iter([
        (
            "fromIntentUuid".into(),
            serde_json::Value::String(from.intent_id().as_str().into()),
        ),
        (
            "toIntentUuid".into(),
            serde_json::Value::String(to.intent_id().as_str().into()),
        ),
        (
            "issuedAtMs".into(),
            serde_json::Value::Number(chrono::Utc::now().timestamp_millis().into()),
        ),
    ]));
    fs::write(
        &handoff,
        core_infrastructure::canon_json::serialize(
            &core_infrastructure::canon_json::to_value(&body).unwrap(),
            core_infrastructure::canon_json::SerializationProfile::ContractPretty,
        ),
    )
    .unwrap();
    let stopped = workspace.stop(r#"{"session_id":"handoff-session"}"#, &[]);
    assert!(
        stopped.stdout.is_empty() && stopped.stderr.is_empty(),
        "{stopped:?}"
    );
    assert!(!handoff.exists());
    assert_eq!(
        fs::read_to_string(current.join(".aidlc-stop-hook/block-count.json")).unwrap(),
        r#"{"signature":"","count":0}"#
    );
    let continued = workspace.stop(r#"{"session_id":"handoff-session"}"#, &[]);
    assert!(
        !continued.stdout.is_empty(),
        "handoff受領を再利用しない: {continued:?}"
    );
}

#[test]
fn a_parked_workflow_resets_the_persisted_stop_counter() {
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    assert!(!workspace.stop("{}", &[]).stdout.is_empty());
    let parked = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .arg("park")
        .current_dir(workspace.path())
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", workspace.path())
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    assert!(parked.status.success(), "{parked:?}");
    let output = workspace.stop("{}", &[]);
    assert_eq!(output.status.code(), Some(0));
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "{output:?}"
    );
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    assert_eq!(
        fs::read_to_string(record.join(".aidlc-stop-hook/block-count.json")).unwrap(),
        r#"{"signature":"","count":0}"#
    );
}

#[test]
fn a_summary_question_requires_a_human_response_after_its_prompt() {
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    assert!(
        workspace
            .human_prompt(r#"{"session_id":"s","prompt":"Looks correct"}"#, false)
            .status
            .success()
    );
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let active = fs::read_to_string(intents.join("active-intent")).unwrap();
    let relative = format!(
        "aidlc/spaces/default/intents/{}/requirements-analysis-questions.md",
        active.trim()
    );
    let body = "# Questions\n\n## Consolidated Summary Confirmation\n[Answer]:\n";
    fs::write(workspace.path().join(&relative), body).unwrap();
    let asked = workspace.log(&[
        "decision",
        "--stage",
        "requirements-analysis",
        "--checkpoint",
        "summary-confirmation",
        "--questions-file",
        &relative,
        "--decision",
        "Confirm summary",
    ]);
    assert!(asked.status.success(), "{asked:?}");
    fs::write(
        workspace.path().join(&relative),
        body.replace("[Answer]:", "[Answer]: Looks correct"),
    )
    .unwrap();
    let refused = workspace.log(&[
        "answer",
        "--stage",
        "requirements-analysis",
        "--checkpoint",
        "summary-confirmation",
        "--questions-file",
        &relative,
        "--details",
        "Looks correct",
    ]);
    assert!(
        !refused.status.success(),
        "提示前の発言は使わない: {refused:?}"
    );
    assert_eq!(
        String::from_utf8(refused.stderr).unwrap(),
        upstream_error(
            "Cannot record the summary choice because no human reply has arrived after this question, or that turn was already used by another decision. End the turn, wait for the human's choice, then try again."
        )
    );
    assert!(
        workspace
            .human_prompt(r#"{"session_id":"s","prompt":"Looks correct"}"#, false)
            .status
            .success()
    );
    let accepted = workspace.log(&[
        "answer",
        "--stage",
        "requirements-analysis",
        "--checkpoint",
        "summary-confirmation",
        "--questions-file",
        &relative,
        "--details",
        "Looks correct",
    ]);
    assert!(accepted.status.success(), "{accepted:?}");
    let repeated = workspace.log(&[
        "answer",
        "--stage",
        "requirements-analysis",
        "--checkpoint",
        "summary-confirmation",
        "--questions-file",
        &relative,
        "--details",
        "Looks correct",
    ]);
    assert!(!repeated.status.success());
    assert!(
        String::from_utf8(repeated.stderr)
            .unwrap()
            .contains("no matching unanswered summary question")
    );
}

#[test]
fn summary_confirmation_binds_the_reviewed_questions_and_a_later_human_reply() {
    use base64::Engine as _;
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/upstream-a277af21/stage1/cases.json"
    ))
    .unwrap();
    let observations = corpus.get("observations").unwrap().as_array().unwrap();
    let case = |id| {
        observations
            .iter()
            .find(|case| case.get("id").and_then(serde_json::Value::as_str) == Some(id))
            .unwrap()
    };
    let decision = case("summary/decision");
    let original = decision
        .get("initial_files")
        .unwrap()
        .as_object()
        .unwrap()
        .iter()
        .find(|(path, _)| path.ends_with("requirements-analysis-questions.md"))
        .unwrap()
        .1
        .as_str()
        .unwrap();
    let original = String::from_utf8(
        base64::engine::general_purpose::STANDARD
            .decode(original)
            .unwrap(),
    )
    .unwrap();
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let active = fs::read_to_string(intents.join("active-intent")).unwrap();
    let relative = format!(
        "aidlc/spaces/default/intents/{}/inception/requirements-analysis/requirements-analysis-questions.md",
        active.trim()
    );
    let path = workspace.path().join(&relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, &original).unwrap();
    let asked = workspace.log(&[
        "decision",
        "--stage",
        "requirements-analysis",
        "--checkpoint",
        "summary-confirmation",
        "--questions-file",
        &relative,
        "--decision",
        "Does this all look correct before I generate the artifact?",
        "--options",
        "Looks correct,Request changes",
    ]);
    assert!(asked.status.success(), "{asked:?}");
    assert_eq!(
        String::from_utf8(asked.stdout).unwrap(),
        decision
            .get("output")
            .unwrap()
            .get("stdout")
            .unwrap()
            .as_str()
            .unwrap()
    );
    assert!(
        workspace
            .human_prompt(
                r#"{"session_id":"stage1-session","prompt":"Looks correct"}"#,
                false
            )
            .status
            .success()
    );
    fs::write(
        &path,
        original.replace("[Answer]:\n", "[Answer]: Looks correct\n"),
    )
    .unwrap();
    let answered = workspace.log(&[
        "answer",
        "--stage",
        "requirements-analysis",
        "--checkpoint",
        "summary-confirmation",
        "--questions-file",
        &relative,
        "--details",
        "Looks correct",
    ]);
    assert!(answered.status.success(), "{answered:?}");
    let expected = case("summary/answer");
    assert_eq!(
        String::from_utf8(answered.stdout).unwrap(),
        expected
            .get("output")
            .unwrap()
            .get("stdout")
            .unwrap()
            .as_str()
            .unwrap()
    );
    let expected_audit = expected
        .get("changed_files")
        .unwrap()
        .as_object()
        .unwrap()
        .iter()
        .find(|(path, _)| path.contains("/audit/"))
        .unwrap()
        .1;
    let expected_audit = expected_audit.as_str().unwrap();
    let expected_audit = String::from_utf8(
        base64::engine::general_purpose::STANDARD
            .decode(expected_audit)
            .unwrap(),
    )
    .unwrap();
    let fingerprint = expected_audit
        .rsplit("**Questions SHA-256**: ")
        .next()
        .unwrap()
        .lines()
        .next()
        .unwrap();
    assert_eq!(fingerprint.len(), 64);
    let audit = fs::read_to_string(
        fs::read_dir(intents.join(active.trim()).join("audit"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path(),
    )
    .unwrap();
    assert!(
        audit.contains(&format!("**Questions SHA-256**: {fingerprint}\n")),
        "{audit}"
    );
    assert!(audit.contains("**Event**: SUMMARY_CONFIRMATION_RECORDED\n"));
    assert!(audit.contains("**Hash Scope**: confirmed-content-v1\n"));
}

#[test]
fn a_dismissed_widget_is_not_an_answer_and_does_not_consume_presence() {
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    assert!(
        workspace
            .human_prompt(r#"{"session_id":"s","prompt":"Cancelled"}"#, false)
            .status
            .success()
    );
    let refused = workspace.log(&[
        "answer",
        "--stage",
        "requirements-analysis",
        "--details",
        "Cancelled",
    ]);
    assert!(!refused.status.success(), "取消を回答にしない: {refused:?}");
    assert_eq!(
        String::from_utf8(refused.stderr).unwrap(),
        upstream_error(
            "Cannot record reply \"Cancelled\" because it represents a dismissed question, not a human answer. Re-present the question and wait for a real response before trying again."
        )
    );
    let store = workspace
        .path()
        .join("aidlc/spaces/default/intents/.aidlc-store.sqlite");
    let db = rusqlite::Connection::open(store).unwrap();
    let count: i64 = db
        .query_row("SELECT count(*) FROM read_answer_result", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 0);
    let consumed: i64 = db.query_row("SELECT count(*) FROM snapshot WHERE json_extract(CAST(payload AS TEXT), '$.interactions.consumed_human') IS NOT NULL", [], |row| row.get(0)).unwrap();
    assert_eq!(consumed, 0);
}

#[test]
fn an_approval_answer_without_a_human_reply_has_the_upstream_refusal() {
    let workspace = Workspace::new();
    fs::write(workspace.path().join("main.ts"), "export const a = 1;\n").unwrap();
    assert!(workspace.create().status.success());
    workspace.complete_reverse_engineering_pipeline();
    let opened = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .args(["report", "--result", "awaiting-approval"])
        .current_dir(workspace.path())
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", workspace.path())
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    assert!(opened.status.success());
    assert!(
        workspace.state().contains("- [?] reverse-engineering"),
        "承認ゲートが開いていなければ承認分岐に立たない: {opened:?}"
    );
    let refused = workspace.log(&[
        "answer",
        "--stage",
        "reverse-engineering",
        "--details",
        "Approve",
    ]);
    assert!(!refused.status.success());
    assert_eq!(
        String::from_utf8(refused.stderr).unwrap(),
        upstream_error(
            "Cannot record this approval choice because no new human reply has arrived. After the human types their choice, use aidlc-orchestrate.ts report --result approved or rejected; do not use aidlc-log.ts answer for an approval."
        )
    );
}

#[test]
fn an_approval_choice_is_acknowledged_without_consuming_the_human_response() {
    let workspace = Workspace::new();
    fs::write(workspace.path().join("main.ts"), "export const a = 1;\n").unwrap();
    assert!(workspace.create().status.success());
    assert!(
        workspace
            .log(&[
                "decision",
                "--stage",
                "reverse-engineering",
                "--decision",
                "Old question"
            ])
            .status
            .success()
    );
    workspace.complete_reverse_engineering_pipeline();
    let opened = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .args(["report", "--result", "awaiting-approval"])
        .current_dir(workspace.path())
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", workspace.path())
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    assert!(opened.status.success(), "{opened:?}");
    assert!(
        workspace.state().contains("- [?] reverse-engineering"),
        "承認ゲートが開いていなければ承認分岐に立たない: {opened:?}"
    );
    assert!(
        workspace
            .human_prompt(r#"{"session_id":"s","prompt":"1"}"#, false)
            .status
            .success()
    );
    let before = workspace.state();
    let answer = workspace.log(&[
        "answer",
        "--stage",
        "reverse-engineering",
        "--details",
        "Approve",
    ]);
    assert!(answer.status.success(), "{answer:?}");
    assert_eq!(
        String::from_utf8(answer.stdout).unwrap(),
        "{\"skipped\":\"QUESTION_ANSWERED\",\"stage\":\"reverse-engineering\",\"reason\":\"approval-gate-report-owned\"}\n"
    );
    assert_eq!(workspace.state(), before);
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let execution = fs::read_to_string(record.join(".aidlc-execution"))
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_string();
    let db = rusqlite::Connection::open(intents.join(".aidlc-store.sqlite")).unwrap();
    let consumed: Option<String> = db.query_row("SELECT json_extract(CAST(payload AS TEXT), '$.interactions.consumed_human') FROM snapshot WHERE aid=?1", [&execution], |row| row.get(0)).unwrap();
    assert_eq!(consumed, None, "承認のreport用に応答を残す");
    let audit = fs::read_to_string(
        fs::read_dir(record.join("audit"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path(),
    )
    .unwrap();
    assert!(!audit.contains("**Event**: QUESTION_ANSWERED"));
}

#[test]
fn a_saved_answer_remains_consumed_when_projection_fails_and_restarts() {
    use core_query_interface_adapter::ReadModelDaos;
    use core_query_use_case::orchestration::AnswerResultUseCase;
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    assert!(
        workspace
            .log(&[
                "decision",
                "--stage",
                "requirements-analysis",
                "--decision",
                "修正対象は何か。"
            ])
            .status
            .success()
    );
    assert!(
        workspace
            .human_prompt(r#"{"session_id":"s","prompt":"A"}"#, false)
            .status
            .success()
    );
    let store = workspace
        .path()
        .join("aidlc/spaces/default/intents/.aidlc-store.sqlite");
    let db = rusqlite::Connection::open(&store).unwrap();
    db.execute_batch("CREATE TRIGGER stop_answer_publication BEFORE INSERT ON amadeus_publication BEGIN SELECT RAISE(ABORT,'answer projection unavailable'); END;").unwrap();
    let failed = workspace.log(&[
        "answer",
        "--stage",
        "requirements-analysis",
        "--details",
        "A",
    ]);
    assert!(
        !failed.status.success(),
        "投影失敗を成功にしない: {failed:?}"
    );
    let answer_id: String = db.query_row("SELECT json_extract(CAST(payload AS TEXT), '$.AnswerRecorded.answer_id') FROM journal WHERE json_extract(CAST(payload AS TEXT), '$.AnswerRecorded') IS NOT NULL", [], |row| row.get(0)).unwrap();
    let query = || {
        AnswerResultUseCase::new(ReadModelDaos::open(&store).unwrap().answer_result())
            .execute(&answer_id)
            .unwrap()
    };
    assert!(
        query().is_none(),
        "保存されても未投影の間は結果を創作しない"
    );
    db.execute_batch("DROP TRIGGER stop_answer_publication;")
        .unwrap();
    let retried = workspace.log(&[
        "answer",
        "--stage",
        "requirements-analysis",
        "--details",
        "A",
    ]);
    assert!(
        !retried.status.success(),
        "再起動しても消費済み: {retried:?}"
    );
    assert!(
        String::from_utf8(retried.stderr)
            .unwrap()
            .contains("no new human reply")
    );
    let recovered = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .arg("next")
        .current_dir(workspace.path())
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", workspace.path())
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    assert!(recovered.status.success(), "{recovered:?}");
    let result = query().unwrap();
    assert_eq!(result.stage(), "requirements-analysis");
    assert_eq!(result.disposition(), "recorded");
    assert!(
        workspace
            .human_prompt(r#"{"session_id":"s","prompt":"B"}"#, false)
            .status
            .success()
    );
    assert!(
        workspace
            .log(&["answer", "--stage", "reverse-engineering", "--details", "B"])
            .status
            .success()
    );
    assert_eq!(
        query(),
        Some(result),
        "後続の回答で元のAnswerIdの結果を置き換えない"
    );
}

#[test]
fn a_handwritten_human_audit_entry_cannot_authorize_an_answer() {
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let path = fs::read_dir(record.join("audit"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let mut audit = fs::read_to_string(&path).unwrap();
    audit.push_str("\n## Human Turn\n**Timestamp**: 2099-01-01T00:00:00Z\n**Event**: HUMAN_TURN\n**Session**: s\n\n---\n");
    fs::write(&path, &audit).unwrap();
    let before = workspace.state();
    let refused = workspace.log(&[
        "answer",
        "--stage",
        "requirements-analysis",
        "--details",
        "A",
    ]);
    assert!(
        !refused.status.success(),
        "監査の手書きは権限を作らない: {refused:?}"
    );
    assert!(
        String::from_utf8(refused.stderr)
            .unwrap()
            .contains("no new human reply")
    );
    // 本家 `emitError`（aidlc-lib.ts:22219-22245）は状態ファイルがあれば拒否のたびに
    // `ERROR_LOGGED` を監査へ追記する。手書き行は残り、回答の受領証は増えない。
    let after = fs::read_to_string(path).unwrap();
    let appended = after
        .strip_prefix(&audit)
        .expect("監査は追記専用: 手書き行の後ろに拒否行だけが加わる");
    assert_eq!(
        appended.matches("**Event**: ERROR_LOGGED").count(),
        1,
        "{appended}"
    );
    assert!(appended.contains("**Tool**: aidlc-log"), "{appended}");
    assert!(
        appended.contains("**Error**: Cannot record this answer because no new human reply"),
        "{appended}"
    );
    assert!(!after.contains("**Event**: QUESTION_ANSWERED"));
    assert_eq!(workspace.state(), before);
}

#[test]
fn an_answer_consumes_one_recorded_human_response_across_processes() {
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    let asked = workspace.log(&[
        "decision",
        "--stage",
        "requirements-analysis",
        "--decision",
        "修正対象は何か。",
        "--options",
        "小さな不具合,Other",
    ]);
    assert!(asked.status.success(), "{asked:?}");
    let human = workspace.human_prompt(
        r#"{"hook_event_name":"UserPromptSubmit","session_id":"stage1-session","prompt":"A"}"#,
        false,
    );
    assert!(human.status.success(), "{human:?}");
    let before = workspace.state();
    let answered = workspace.log(&[
        "answer",
        "--stage",
        "requirements-analysis",
        "--details",
        "A",
    ]);
    assert!(answered.status.success(), "{answered:?}");
    assert_eq!(
        String::from_utf8(answered.stdout).unwrap(),
        "{\"emitted\":\"QUESTION_ANSWERED\",\"stage\":\"requirements-analysis\"}\n"
    );
    assert!(answered.stderr.is_empty());
    assert_eq!(workspace.state(), before);
    let repeated = workspace.log(&[
        "answer",
        "--stage",
        "requirements-analysis",
        "--details",
        "A",
    ]);
    assert!(
        !repeated.status.success(),
        "同じ人間応答を再利用しない: {repeated:?}"
    );
    assert_eq!(
        String::from_utf8(repeated.stderr).unwrap(),
        upstream_error(
            "Cannot record this answer because no new human reply has arrived for the question. Wait for the human to type an answer, then try again."
        )
    );
    let db = rusqlite::Connection::open(
        workspace
            .path()
            .join("aidlc/spaces/default/intents/.aidlc-store.sqlite"),
    )
    .unwrap();
    let rows: i64 = db
        .query_row("SELECT count(*) FROM read_answer_result", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(rows, 1, "拒否では成功結果を作らない");
}

#[test]
fn a_human_prompt_hook_records_presence_and_preserves_a_numeric_response() {
    use std::io::Write as _;
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    let before = workspace.state();
    let mut child = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .args(["hook", "record-human-turn"])
        .current_dir(workspace.path())
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", workspace.path())
        .env("PATH", "/usr/bin:/bin")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            br#"{"hook_event_name":"UserPromptSubmit","session_id":"stage1-session","prompt":"1"}"#,
        )
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    assert_eq!(workspace.state(), before);
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let audit = fs::read_to_string(
        fs::read_dir(record.join("audit"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path(),
    )
    .unwrap();
    assert_eq!(audit.matches("**Event**: HUMAN_TURN").count(), 1);
    assert!(audit.contains("**Session**: stage1-session\n"));
    assert!(record.join(".aidlc-human-turn").is_file());
    let db = rusqlite::Connection::open(intents.join(".aidlc-store.sqlite")).unwrap();
    let response: String = db.query_row("SELECT json_extract(CAST(payload AS TEXT), '$.PromptObserved.response') FROM journal WHERE json_extract(CAST(payload AS TEXT), '$.PromptObserved') IS NOT NULL", [], |row| row.get(0)).unwrap();
    assert_eq!(response, "1", "番号応答の原文を保持する");
}

#[test]
fn an_ordinary_decision_is_persisted_and_projected_without_changing_workflow_state() {
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    let log = workspace.path().join("bin/aidlc-log");
    fs::copy(env!("CARGO_BIN_EXE_aidlc"), &log).unwrap();
    let before = workspace.state();
    let source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/upstream-a277af21/stage1/cases.json"
    ))
    .unwrap();
    let case = source
        .get("observations")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|case| {
            case.get("id").and_then(serde_json::Value::as_str) == Some("question/decision")
        })
        .unwrap();
    let args: Vec<&str> = case
        .get("input")
        .unwrap()
        .get("argv")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .skip(2)
        .map(|value| value.as_str().unwrap())
        .collect();
    let output = Command::new(log)
        .args(args)
        .current_dir(workspace.path())
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", workspace.path())
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        case.get("output")
            .unwrap()
            .get("stdout")
            .unwrap()
            .as_str()
            .unwrap()
    );
    assert!(output.stderr.is_empty());
    assert_eq!(workspace.state(), before);
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let audit = fs::read_to_string(
        fs::read_dir(record.join("audit"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path(),
    )
    .unwrap();
    assert_eq!(audit.matches("**Event**: DECISION_RECORDED").count(), 1);
    assert!(audit.contains("**Stage**: requirements-analysis\n**Decision**: 修正対象は何か。\n**Options**: 小さな不具合,Other\n"), "{audit}");
    let db = rusqlite::Connection::open(intents.join(".aidlc-store.sqlite")).unwrap();
    let count: i64 = db.query_row("SELECT count(*) FROM journal WHERE json_extract(CAST(payload AS TEXT), '$.DecisionRecorded') IS NOT NULL", [], |row| row.get(0)).unwrap();
    assert_eq!(count, 1);
}

#[test]
fn a_rust_workspace_starts_the_brownfield_bugfix_path() {
    let workspace = Workspace::new();
    fs::write(
        workspace.path().join("Cargo.toml"),
        "[package]\nname='fixture'\nversion='0.1.0'\n",
    )
    .unwrap();
    fs::create_dir_all(workspace.path().join("src")).unwrap();
    fs::write(workspace.path().join("src/lib.rs"), "pub fn example() {}\n").unwrap();
    let output = workspace.create();
    assert!(output.status.success(), "{output:?}");
    let state = workspace.state();
    assert!(state.contains("- **Project Type**: Brownfield"), "{state}");
    assert!(state.contains("- **Languages**: Rust"), "{state}");
    assert!(
        state.contains("- **Build System**: cargo (Cargo.toml)"),
        "{state}"
    );
    assert!(state.contains("- [-] reverse-engineering"), "{state}");
}

#[test]
fn source_languages_are_counted_and_distribution_directories_are_excluded() {
    let workspace = Workspace::new();
    fs::write(
        workspace.path().join("main.ts"),
        "export const example = 1;\n",
    )
    .unwrap();
    fs::create_dir_all(workspace.path().join("vendor")).unwrap();
    for i in 0..10 {
        fs::write(
            workspace.path().join(format!("vendor/{i}.rs")),
            "pub fn vendor() {}\n",
        )
        .unwrap();
    }
    let output = workspace.create();
    assert!(output.status.success(), "{output:?}");
    let state = workspace.state();
    assert!(state.contains("- **Project Type**: Brownfield"), "{state}");
    assert!(state.contains("- **Languages**: TypeScript\n"), "{state}");
}

#[test]
fn bugfix_initial_state_matches_the_captured_271_bytes() {
    use base64::Engine as _;
    let workspace = Workspace::new();
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/upstream-a277af21/stage1/cases.json"
    ))
    .unwrap();
    let case = corpus
        .get("observations")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|case| {
            case.get("id").and_then(serde_json::Value::as_str) == Some("intent-create/bugfix")
        })
        .unwrap();
    for (path, encoded) in case.get("initial_files").unwrap().as_object().unwrap() {
        let target = workspace.path().join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(
            target,
            base64::engine::general_purpose::STANDARD
                .decode(encoded.as_str().unwrap())
                .unwrap(),
        )
        .unwrap();
    }
    let args: Vec<&str> = case
        .get("input")
        .unwrap()
        .get("argv")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .skip(2)
        .map(|v| v.as_str().unwrap())
        .collect();
    let output = Command::new(workspace.path().join("bin/aidlc-utility"))
        .args(args)
        .current_dir(workspace.path())
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", workspace.path())
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let expected = case
        .get("changed_files")
        .unwrap()
        .as_object()
        .unwrap()
        .iter()
        .find(|(path, _)| path.ends_with("/aidlc-state.md"))
        .unwrap()
        .1
        .as_str()
        .unwrap();
    let expected = String::from_utf8(
        base64::engine::general_purpose::STANDARD
            .decode(expected)
            .unwrap(),
    )
    .unwrap();
    fn normalize_time(state: &str) -> String {
        let started = state
            .lines()
            .find_map(|line| line.strip_prefix("- **Start Date**: "))
            .unwrap();
        assert_eq!(
            state.matches(started).count(),
            2,
            "Start DateとLast Updatedの対応を維持する"
        );
        state.replace(started, "<TIMESTAMP>")
    }
    assert_eq!(
        normalize_time(&workspace.state()),
        normalize_time(&expected)
    );
    let record = fs::read_to_string(
        workspace
            .path()
            .join("aidlc/spaces/default/intents/active-intent"),
    )
    .unwrap();
    assert_eq!(
        record.trim().split_once('-').unwrap().1,
        "stage1",
        "初回の記録名にid8は付かない"
    );
    let captured_stdout = case
        .get("output")
        .unwrap()
        .get("stdout")
        .unwrap()
        .as_str()
        .unwrap();
    let captured_record = captured_stdout
        .lines()
        .next()
        .unwrap()
        .strip_prefix("Intent created: ")
        .unwrap()
        .strip_suffix(" (space: default)")
        .unwrap();
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        captured_stdout.replace(captured_record, record.trim())
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn next_doctor_routes_to_the_exact_terminal_utility_before_workflow_reads() {
    let root = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .args(["next", "--doctor"])
        .current_dir(root.path())
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", root.path())
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    let directive: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        directive.get("kind").and_then(serde_json::Value::as_str),
        Some("print")
    );
    // 案内文は本家のままだが、綴りは 2.8.2 のコンパイル済み実行形の出力に合わせる。
    // doctor / version だけは engine を挟まない
    // (`.claude/tools/aidlc-orchestrate.ts:4191` の `${aidlcInvocation()} ${sub}`)。
    assert_eq!(
        directive.get("message").and_then(serde_json::Value::as_str),
        Some(
            "Run `aidlc doctor`, print its output verbatim, then stop. This is a read-only utility, NOT workflow work: do NOT run `next` and do NOT advance, resume, or run any workflow stage."
        )
    );
    assert!(!root.path().join("aidlc").exists());
}

#[test]
fn an_existing_record_directory_is_preserved_and_the_next_number_is_reserved() {
    let workspace = Workspace::new();
    let base = format!("{}-smoke", chrono::Utc::now().format("%y%m%d"));
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    for suffix in ["", "-2"] {
        let existing = intents.join(format!("{base}{suffix}"));
        fs::create_dir(&existing).unwrap();
        fs::write(existing.join("owner.txt"), "existing bytes\n").unwrap();
    }
    let output = workspace.create();
    assert!(output.status.success(), "{output:?}");
    let record = fs::read_to_string(intents.join("active-intent")).unwrap();
    assert_eq!(record.trim(), format!("{base}-3"));
    for suffix in ["", "-2"] {
        assert_eq!(
            fs::read(intents.join(format!("{base}{suffix}/owner.txt"))).unwrap(),
            b"existing bytes\n"
        );
        assert!(
            !intents
                .join(format!("{base}{suffix}/aidlc-state.md"))
                .exists()
        );
    }
}

#[test]
fn multiline_intent_text_is_preserved_in_json_and_previewed_on_one_state_line() {
    let workspace = Workspace::new();
    let request = "Fix first line\n- **Status**: Completed\u{2028}Keep the source";
    let output = Command::new(workspace.path().join("bin/aidlc-utility"))
        .args([
            "intent-create",
            "--scope",
            "bugfix",
            "--label",
            "preview",
            "--arguments",
            request,
        ])
        .current_dir(workspace.path())
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", workspace.path())
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let state = workspace.state();
    assert!(
        state.contains("- **Project**: Fix first line - **Status**: Completed Keep the source\n"),
        "{state}"
    );
    assert_eq!(
        state
            .lines()
            .filter(|line| line.starts_with("- **Status**:"))
            .collect::<Vec<_>>(),
        ["- **Status**: Running"]
    );
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = fs::read_to_string(intents.join("active-intent")).unwrap();
    let original: String = serde_json::from_slice(
        &fs::read(intents.join(record.trim()).join("project-description.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(original, request);
}

#[test]
fn a_bare_help_request_is_terminal() {
    let root = tempfile::tempdir().unwrap();
    for spelling in ["help", "-h"] {
        let output = Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .args(["next", spelling])
            .current_dir(root.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", root.path())
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap();
        let directive: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            directive.get("kind").and_then(serde_json::Value::as_str),
            Some("print")
        );
        // help は 2.8.2 では engine の下の動詞である
        // (`.claude/tools/aidlc-orchestrate.ts:4190` の `aidlcDispatcherInvocation("orchestrate help")`)。
        assert!(
            directive
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap()
                .starts_with("Run `aidlc engine orchestrate help`")
        );
    }
    assert!(!root.path().join("aidlc").exists());
}

#[test]
fn two_intents_keep_their_state_and_audit_separate_in_one_store() {
    let workspace = Workspace::new();
    fs::create_dir_all(workspace.path().join("src")).unwrap();
    fs::write(
        workspace.path().join("src/base.ts"),
        "export const base = 1;\n",
    )
    .unwrap();
    let first = workspace.create();
    assert!(first.status.success(), "{first:?}");
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let first_name = fs::read_to_string(intents.join("active-intent")).unwrap();
    let first_record = intents.join(first_name.trim());
    let first_state = fs::read(first_record.join("aidlc-state.md")).unwrap();
    let audit_path = fs::read_dir(first_record.join("audit"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let first_audit = fs::read(&audit_path).unwrap();
    let second = workspace.create();
    assert!(
        second.status.success(),
        "2件目の作成: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    let second_name = fs::read_to_string(intents.join("active-intent")).unwrap();
    assert_ne!(first_name, second_name);
    assert!(second_name.trim().ends_with("-smoke-2"));
    assert_eq!(
        fs::read(first_record.join("aidlc-state.md")).unwrap(),
        first_state
    );
    assert_eq!(fs::read(&audit_path).unwrap(), first_audit);
    fs::write(intents.join("active-intent"), &first_name).unwrap();
    let next = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .arg("next")
        .current_dir(workspace.path())
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", workspace.path())
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    assert!(next.status.success(), "{next:?}");
    let directive: serde_json::Value = serde_json::from_slice(&next.stdout).unwrap();
    assert_ne!(
        directive.get("kind").and_then(serde_json::Value::as_str),
        Some("error"),
        "{directive}"
    );
    assert_eq!(
        fs::read(first_record.join("aidlc-state.md")).unwrap(),
        first_state
    );
    assert_eq!(fs::read(&audit_path).unwrap(), first_audit);
}

#[test]
fn different_scopes_preserve_report_results_through_another_intents_publication_failure() {
    use core_query_interface_adapter::ReadModelDaos;
    use core_query_use_case::orchestration::ReportResultUseCase;
    let workspace = Workspace::new();
    fs::create_dir_all(workspace.path().join("src")).unwrap();
    fs::write(
        workspace.path().join("src/base.ts"),
        "export const base = 1;\n",
    )
    .unwrap();
    fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.claude/scopes/aidlc-feature.md"),
        workspace.path().join(".claude/scopes/aidlc-feature.md"),
    )
    .unwrap();
    // 投影の失敗を確かめる試験なので、要約確認のガード（先に断る）を外す。
    let run = |binary: &std::path::Path, args: &[&str]| {
        Command::new(binary)
            .args(args)
            .current_dir(workspace.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", workspace.path())
            .env("PATH", "/usr/bin:/bin")
            .env("AIDLC_SKIP_SUMMARY_CONFIRMATION_GUARD", "1")
            .output()
            .unwrap()
    };
    let engine = std::path::Path::new(env!("CARGO_BIN_EXE_aidlc"));
    assert!(workspace.create().status.success());
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let first = fs::read_to_string(intents.join("active-intent")).unwrap();
    workspace.complete_reverse_engineering_pipeline();
    let reported = run(engine, &["report", "--result", "awaiting-approval"]);
    assert!(reported.status.success(), "{reported:?}");
    assert!(
        workspace.state().contains("- [?] reverse-engineering"),
        "承認ゲートが開かなければ報告結果の行は生まれない: {reported:?}"
    );
    let first_state = fs::read(intents.join(first.trim()).join("aidlc-state.md")).unwrap();
    let audit = |name: &str| {
        fs::read_dir(intents.join(name.trim()).join("audit"))
            .unwrap()
            .map(|entry| fs::read(entry.unwrap().path()).unwrap())
            .collect::<Vec<_>>()
    };
    let first_audit = audit(&first);
    let store = intents.join(".aidlc-store.sqlite");
    let database = rusqlite::Connection::open(&store).unwrap();
    let report_id: String = database
        .query_row("SELECT report_id FROM read_report_result", [], |row| {
            row.get(0)
        })
        .unwrap();
    let read_result = || {
        ReportResultUseCase::new(ReadModelDaos::open(&store).unwrap().report_result())
            .execute(&report_id)
            .unwrap()
            .unwrap()
    };
    let first_result = read_result();
    assert_eq!(first_result.scope(), "bugfix");
    let second = run(
        &workspace.path().join("bin/aidlc-utility"),
        &[
            "intent-create",
            "--scope",
            "feature",
            "--label",
            "another",
            "--arguments",
            "Add another feature",
        ],
    );
    assert!(second.status.success(), "{second:?}");
    let second = fs::read_to_string(intents.join("active-intent")).unwrap();
    let second_record = intents.join(second.trim());
    let second_state = fs::read(second_record.join("aidlc-state.md")).unwrap();
    assert!(
        String::from_utf8(second_state.clone())
            .unwrap()
            .contains("- **Scope**: feature")
    );
    assert!(
        fs::read_to_string(second_record.join("project-description.json"))
            .unwrap()
            .contains("Add another feature")
    );
    let second_audit = audit(&second);
    database.execute_batch("CREATE TRIGGER stop_publication BEFORE INSERT ON amadeus_publication BEGIN SELECT RAISE(ABORT,'publication stopped'); END;").unwrap();
    let failed = run(engine, &["report", "--result", "awaiting-approval"]);
    assert!(
        !failed.status.success(),
        "投影失敗は成功を返さない: {failed:?}"
    );
    assert_eq!(
        fs::read(second_record.join("aidlc-state.md")).unwrap(),
        second_state
    );
    assert_eq!(audit(&second), second_audit);
    database
        .execute_batch("DROP TRIGGER stop_publication;")
        .unwrap();
    fs::write(intents.join("active-intent"), &first).unwrap();
    let resumed_first = run(engine, &["next"]);
    assert!(resumed_first.status.success(), "{resumed_first:?}");
    assert_eq!(
        fs::read(intents.join(first.trim()).join("aidlc-state.md")).unwrap(),
        first_state
    );
    assert_eq!(audit(&first), first_audit);
    assert_eq!(read_result(), first_result);
    fs::write(intents.join("active-intent"), &second).unwrap();
    let recovered = run(engine, &["next"]);
    assert!(recovered.status.success(), "{recovered:?}");
    let recovered_state = fs::read(second_record.join("aidlc-state.md")).unwrap();
    assert!(
        std::str::from_utf8(&recovered_state)
            .unwrap()
            .contains("- [?] intent-capture"),
        "{}",
        std::str::from_utf8(&recovered_state).unwrap()
    );
    assert_ne!(recovered_state, second_state);
    let recovered_audit = audit(&second);
    assert_ne!(recovered_audit, second_audit);
    assert!(run(engine, &["next"]).status.success());
    assert_eq!(audit(&second), recovered_audit);
    assert_eq!(read_result(), first_result);
    assert_eq!(
        fs::read(intents.join(first.trim()).join("aidlc-state.md")).unwrap(),
        first_state
    );
    assert_eq!(audit(&first), first_audit);
}

#[test]
fn a_legacy_shared_checkpoint_is_refused_without_replaying_public_files() {
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let state = fs::read(record.join("aidlc-state.md")).unwrap();
    let audit_file = fs::read_dir(record.join("audit"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let audit = fs::read(&audit_file).unwrap();
    let database = rusqlite::Connection::open(intents.join(".aidlc-store.sqlite")).unwrap();
    assert_eq!(
        database
            .execute(
                "UPDATE amadeus_projection_checkpoint SET projection='orchestration'",
                []
            )
            .unwrap(),
        1
    );
    let output = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .arg("next")
        .current_dir(workspace.path())
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", workspace.path())
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    let directive: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        directive.get("kind").and_then(serde_json::Value::as_str),
        Some("error")
    );
    assert!(
        directive
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap()
            .contains("legacy shared projection requires migration: orchestration"),
        "{directive}"
    );
    assert_eq!(fs::read(record.join("aidlc-state.md")).unwrap(), state);
    assert_eq!(fs::read(audit_file).unwrap(), audit);
    let names: Vec<String> = database
        .prepare("SELECT projection FROM amadeus_projection_checkpoint")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(names, vec!["orchestration"]);
}

#[test]
fn native_creation_preserves_existing_typescript_records_and_registration() {
    let workspace = Workspace::new();
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let old = intents.join("260907-existing");
    fs::create_dir_all(old.join("audit")).unwrap();
    let registry = b"[{\"uuid\":\"0190aaaa-bbbb-7ccc-9ddd-eeeeffff0001\",\"slug\":\"existing\",\"dirName\":\"260907-existing\",\"scope\":\"feature\",\"status\":\"in-flight\"}]\n";
    let state = b"# Existing workflow\n\n- **Status**: Running\n";
    let audit = b"# Audit\n\n## Human decision\n**Event**: DECISION\n";
    fs::write(intents.join("intents.json"), registry).unwrap();
    fs::write(old.join("aidlc-state.md"), state).unwrap();
    fs::write(old.join("audit/old-clone.md"), audit).unwrap();
    fs::write(intents.join("active-intent"), "260907-existing\n").unwrap();
    let created = workspace.create();
    assert!(created.status.success(), "{created:?}");
    let rows: serde_json::Value =
        serde_json::from_slice(&fs::read(intents.join("intents.json")).unwrap()).unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 2, "nativeのCreatedもRMUが登録する");
    let original: serde_json::Value = serde_json::from_slice(registry).unwrap();
    assert_eq!(
        rows.first(),
        original.as_array().unwrap().first(),
        "既存TS行の全フィールドを保持する"
    );
    let native = rows.last().unwrap();
    assert_eq!(
        native.get("slug").and_then(serde_json::Value::as_str),
        Some("smoke")
    );
    assert_eq!(
        native.get("scope").and_then(serde_json::Value::as_str),
        Some("bugfix")
    );
    assert_eq!(
        native.get("status").and_then(serde_json::Value::as_str),
        Some("in-flight")
    );
    let active = fs::read_to_string(intents.join("active-intent")).unwrap();
    assert_eq!(
        native.get("dirName").and_then(serde_json::Value::as_str),
        Some(active.trim())
    );
    let cursor = fs::read_to_string(intents.join(active.trim()).join(".aidlc-execution")).unwrap();
    assert_eq!(
        native.get("uuid").and_then(serde_json::Value::as_str),
        cursor.lines().nth(1)
    );
    assert_eq!(fs::read(old.join("aidlc-state.md")).unwrap(), state);
    assert_eq!(fs::read(old.join("audit/old-clone.md")).unwrap(), audit);
    assert!(!old.join(".aidlc-execution").exists());
    assert_ne!(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
        "260907-existing"
    );
}

fn first_reverse_engineering_bytes() -> (Vec<u8>, Vec<u8>) {
    first_reverse_engineering_bytes_with_human(false)
}

#[test]
fn a_human_turn_without_a_state_change_keeps_the_steering_continuation_valid() {
    let (actual, expected) = first_reverse_engineering_bytes_with_human(true);
    assert_eq!(
        String::from_utf8(actual).unwrap(),
        String::from_utf8(expected).unwrap()
    );
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Intervention {
    None,
    Human,
    StateEdit,
    Park,
    Noop,
    OtherIntent,
    SharedKey,
    Marker,
}

fn first_reverse_engineering_bytes_with_human(human_between: bool) -> (Vec<u8>, Vec<u8>) {
    first_reverse_engineering_with_intervention(if human_between {
        Intervention::Human
    } else {
        Intervention::None
    })
}

fn first_reverse_engineering_with_intervention(intervention: Intervention) -> (Vec<u8>, Vec<u8>) {
    use base64::Engine as _;
    let workspace = Workspace::new();
    let capture = match intervention {
        Intervention::Human => {
            include_str!("../../../../tests/golden/selfhost-stage1/steering-human-turn.json")
        }
        Intervention::StateEdit => {
            include_str!("../../../../tests/golden/selfhost-stage1/steering-state-edit.json")
        }
        Intervention::Park => {
            include_str!("../../../../tests/golden/selfhost-stage1/steering-park.json")
        }
        Intervention::Noop => {
            include_str!("../../../../tests/golden/selfhost-stage1/steering-noop.json")
        }
        Intervention::OtherIntent => {
            include_str!("../../../../tests/golden/selfhost-stage1/steering-other-intent.json")
        }
        Intervention::SharedKey => include_str!(
            "../../../../tests/golden/selfhost-stage1/steering-other-intent-shared-key.json"
        ),
        Intervention::None | Intervention::Marker => {
            include_str!("../../../../tests/golden/selfhost-stage1/bugfix-first-next.json")
        }
    };
    let captured: serde_json::Value = serde_json::from_str(capture).unwrap();
    assert_eq!(
        captured
            .get("source")
            .unwrap()
            .get("commit")
            .and_then(serde_json::Value::as_str),
        Some("a277af218f0df7f325d3b8be7b6d90fce2c5bd40")
    );
    let observations = captured.get("observations").unwrap().as_array().unwrap();
    let next = observations
        .iter()
        .find(|case| case.get("id").and_then(serde_json::Value::as_str) == Some("next"))
        .unwrap();
    for (path, content) in next
        .get("initial_files")
        .unwrap()
        .as_object()
        .unwrap()
        .iter()
        .filter(|(path, _)| path.starts_with("aidlc/spaces/default/memory/"))
    {
        let path = workspace.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            path,
            base64::engine::general_purpose::STANDARD
                .decode(content.as_str().unwrap())
                .unwrap(),
        )
        .unwrap();
    }
    for (path, content) in captured
        .get("reference_files")
        .unwrap()
        .as_object()
        .unwrap()
    {
        let path = workspace.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            path,
            base64::engine::general_purpose::STANDARD
                .decode(content.as_str().unwrap())
                .unwrap(),
        )
        .unwrap();
    }
    fs::create_dir_all(workspace.path().join("src")).unwrap();
    fs::write(
        workspace.path().join("src/base.ts"),
        "export const base = 1;\n",
    )
    .unwrap();
    let created = Command::new(workspace.path().join("bin/aidlc-utility"))
        .args([
            "intent-create",
            "--scope",
            "bugfix",
            "--label",
            "first-next",
            "--arguments",
            "Fix one small behavior",
        ])
        .current_dir(workspace.path())
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", workspace.path())
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    assert!(created.status.success(), "{created:?}");
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .args(args)
            .current_dir(workspace.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", workspace.path())
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap()
    };
    let mut output = run(&["next"]);
    let mut directive: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    if intervention == Intervention::Human {
        assert!(
            workspace
                .human_prompt(
                    r#"{"session_id":"steering-session","prompt":"Continue"}"#,
                    false
                )
                .status
                .success()
        );
    }
    if matches!(
        intervention,
        Intervention::StateEdit | Intervention::OtherIntent | Intervention::SharedKey
    ) {
        let intents = workspace.path().join("aidlc/spaces/default/intents");
        let state = || {
            intents
                .join(
                    fs::read_to_string(intents.join("active-intent"))
                        .unwrap()
                        .trim(),
                )
                .join("aidlc-state.md")
        };
        let before = fs::read_to_string(state()).unwrap();
        let original_key = state().with_file_name(".aidlc-steering-token-key");
        if intervention == Intervention::StateEdit {
            fs::write(state(), before + "\n<!-- manual state edit -->\n").unwrap();
        } else {
            let created = Command::new(workspace.path().join("bin/aidlc-utility"))
                .args([
                    "intent-create",
                    "--scope",
                    "bugfix",
                    "--label",
                    "second",
                    "--arguments",
                    "Fix one small behavior",
                ])
                .current_dir(workspace.path())
                .env_clear()
                .envs(coverage_profile_env())
                .env("HOME", workspace.path())
                .env("PATH", "/usr/bin:/bin")
                .output()
                .unwrap();
            assert!(created.status.success(), "{created:?}");
            fs::write(state(), before).unwrap();
            if intervention == Intervention::SharedKey {
                fs::copy(
                    original_key,
                    state().with_file_name(".aidlc-steering-token-key"),
                )
                .unwrap();
            }
        }
    }
    if intervention == Intervention::Park {
        assert!(run(&["park"]).status.success());
    }
    if intervention == Intervention::Noop {
        assert!(
            run(&["report", "--result", "completed", "--stage", "state-init"])
                .status
                .success()
        );
    }
    for _ in 0..20 {
        if directive.get("kind").and_then(serde_json::Value::as_str) != Some("load-steering") {
            break;
        }
        let token = directive
            .get("continue_token")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        output = run(&["continue", &token]);
        directive = serde_json::from_slice(&output.stdout).unwrap();
    }
    let expected = observations
        .last()
        .unwrap()
        .get("output")
        .unwrap()
        .get("stdout")
        .unwrap()
        .as_str()
        .unwrap();
    if matches!(
        intervention,
        Intervention::StateEdit
            | Intervention::Park
            | Intervention::OtherIntent
            | Intervention::SharedKey
    ) {
        return (output.stdout, expected.as_bytes().to_vec());
    }
    let record = fs::read_to_string(
        workspace
            .path()
            .join("aidlc/spaces/default/intents/active-intent"),
    )
    .unwrap();
    if intervention == Intervention::Marker {
        let record_path = workspace
            .path()
            .join("aidlc/spaces/default/intents")
            .join(record.trim());
        let marker_path = record_path.join(".aidlc-active-directive.json");
        assert!(marker_path.is_file(), "指示発行をRMUで保存する");
        let actual: serde_json::Value =
            serde_json::from_slice(&fs::read(marker_path).unwrap()).unwrap();
        let captured_marker = observations
            .last()
            .unwrap()
            .get("changed_files")
            .unwrap()
            .as_object()
            .unwrap()
            .iter()
            .find(|(path, _)| path.ends_with(".aidlc-active-directive.json"))
            .unwrap()
            .1
            .as_str()
            .unwrap();
        let mut expected: serde_json::Value = serde_json::from_slice(
            &base64::engine::general_purpose::STANDARD
                .decode(captured_marker)
                .unwrap(),
        )
        .unwrap();
        let project_sha = core_infrastructure::hash::sha256_hex(
            fs::canonicalize(workspace.path())
                .unwrap()
                .to_str()
                .unwrap()
                .as_bytes(),
        );
        let state_sha = core_infrastructure::hash::sha256_hex(
            &fs::read(record_path.join("aidlc-state.md")).unwrap(),
        );
        let cursor = fs::read_to_string(record_path.join(".aidlc-execution")).unwrap();
        let intent_id = cursor.lines().nth(1).unwrap();
        let owner = format!(
            "sessionless:{}",
            project_sha.chars().take(16).collect::<String>()
        );
        let object = expected.as_object_mut().unwrap();
        object.insert("project_sha256".to_string(), project_sha.into());
        object.insert("intent_uuid".to_string(), intent_id.into());
        object.insert("state_sha256".to_string(), state_sha.clone().into());
        object.insert("owner_session".to_string(), owner.clone().into());
        let attempt = object
            .get_mut("active_attempt")
            .unwrap()
            .as_object_mut()
            .unwrap();
        attempt.insert("command_sha256".to_string(), state_sha.clone().into());
        attempt.insert("issued_state_sha256".to_string(), state_sha.into());
        attempt.insert("session_id".to_string(), owner.into());
        assert_eq!(
            actual, expected,
            "現在の入力に対応する識別子以外を読み替えない"
        );
    }
    let (actual_date, actual_label) = record.trim().split_once('-').unwrap();
    assert_eq!(actual_label, "first-next");
    assert_eq!(actual_date.len(), 6);
    assert!(actual_date.chars().all(|c| c.is_ascii_digit()));
    let captured_creation = observations
        .first()
        .unwrap()
        .get("output")
        .unwrap()
        .get("stdout")
        .unwrap()
        .as_str()
        .unwrap();
    let captured_record = captured_creation
        .lines()
        .next()
        .unwrap()
        .strip_prefix("Intent created: ")
        .unwrap()
        .strip_suffix(" (space: default)")
        .unwrap();
    let (captured_date, captured_label) = captured_record.split_once('-').unwrap();
    assert_eq!(captured_label, actual_label);
    // 日付だけは生成値。同じlabelを保持し、CodeKB名や固定のstage/IDを消さない。
    let expected = expected.replace(
        &format!("intents/{captured_date}-first-next/"),
        &format!("intents/{actual_date}-first-next/"),
    );
    (output.stdout, expected.into_bytes())
}

fn first_reverse_engineering_directives() -> (serde_json::Value, serde_json::Value) {
    let (actual, expected) = first_reverse_engineering_bytes();
    (
        serde_json::from_slice(&actual).unwrap(),
        serde_json::from_slice(&expected).unwrap(),
    )
}

#[test]
fn the_first_reverse_engineering_paths_and_rules_match_upstream() {
    let (actual, expected) = first_reverse_engineering_directives();
    for field in ["stage_file", "memory_path", "produces", "rules_in_context"] {
        assert_eq!(actual.get(field), expected.get(field), "{field}");
    }
}

#[test]
fn the_first_reverse_engineering_pipeline_matches_upstream() {
    let (actual, expected) = first_reverse_engineering_directives();
    assert_eq!(actual.get("pipeline"), expected.get("pipeline"));
}

#[test]
fn the_first_reverse_engineering_directive_matches_fixed_upstream_bytes() {
    let (actual, expected) = first_reverse_engineering_bytes();
    assert_eq!(
        String::from_utf8(actual).unwrap(),
        String::from_utf8(expected).unwrap(),
        "JSONのキー順も含めた実バイト一致"
    );
}

#[test]
fn the_first_reverse_engineering_persona_and_narration_match_upstream() {
    let (actual, expected) = first_reverse_engineering_directives();
    for field in ["conductor_persona", "narration"] {
        assert_eq!(actual.get(field), expected.get(field), "{field}");
    }
}

#[test]
fn a_changed_public_state_body_invalidates_the_steering_continuation() {
    let (actual, expected) = first_reverse_engineering_with_intervention(Intervention::StateEdit);
    assert_eq!(
        String::from_utf8(actual).unwrap(),
        String::from_utf8(expected).unwrap()
    );
}

#[test]
fn a_real_transition_invalidates_the_steering_continuation() {
    let (actual, expected) = first_reverse_engineering_with_intervention(Intervention::Park);
    assert_eq!(
        String::from_utf8(actual).unwrap(),
        String::from_utf8(expected).unwrap()
    );
}

#[test]
fn a_noop_report_keeps_the_steering_continuation_valid() {
    let (actual, expected) = first_reverse_engineering_with_intervention(Intervention::Noop);
    assert_eq!(
        String::from_utf8(actual).unwrap(),
        String::from_utf8(expected).unwrap()
    );
}

#[test]
fn another_intent_with_the_same_state_body_refuses_the_continuation() {
    let (actual, expected) = first_reverse_engineering_with_intervention(Intervention::OtherIntent);
    assert_eq!(
        String::from_utf8(actual).unwrap(),
        String::from_utf8(expected).unwrap()
    );
}

#[test]
fn identical_state_and_shared_key_do_not_authorize_another_intents_continuation() {
    let (actual, expected) = first_reverse_engineering_with_intervention(Intervention::SharedKey);
    assert_eq!(
        String::from_utf8(actual).unwrap(),
        String::from_utf8(expected).unwrap()
    );
}

#[test]
fn emitting_a_stage_persists_the_active_directive_through_the_rmu() {
    first_reverse_engineering_with_intervention(Intervention::Marker);
}

#[test]
fn stop_probe_preserves_the_directive_revision_and_event_history() {
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    let next = |probe: bool| {
        Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .arg("next")
            .current_dir(workspace.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", workspace.path())
            .env("PATH", "/usr/bin:/bin")
            .env("AIDLC_STOP_HOOK_PROBE", if probe { "1" } else { "0" })
            .output()
            .unwrap()
    };
    let issued = next(false);
    assert!(issued.status.success(), "{issued:?}");
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let marker = record.join(".aidlc-active-directive.json");
    let before = fs::read(&marker).unwrap();
    let state = fs::read(record.join("aidlc-state.md")).unwrap();
    let database = rusqlite::Connection::open(intents.join(".aidlc-store.sqlite")).unwrap();
    let event_count = || {
        database
            .query_row("SELECT count(*) FROM journal", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap()
    };
    let before_count = event_count();
    let probed = next(true);
    assert!(probed.status.success(), "{probed:?}");
    assert_eq!(probed.stdout, issued.stdout);
    assert_eq!(
        fs::read(&marker).unwrap(),
        before,
        "Stop確認で発行revisionを更新しない"
    );
    assert_eq!(fs::read(record.join("aidlc-state.md")).unwrap(), state);
    assert_eq!(
        event_count(),
        before_count,
        "Stop確認でイベントを追記しない"
    );
}

fn workspace_at_code_generation() -> Workspace {
    let workspace = Workspace::new();
    // この fixture ではバイナリが Source Baseline の走査対象（ワークスペース内の実ファイル）
    // なので、リンクではなく実体を置く。リンクだと採取が `unbindable` になり床の sha が立たない。
    fs::copy(
        env!("CARGO_BIN_EXE_aidlc"),
        workspace.path().join("bin/aidlc-testing-posture"),
    )
    .unwrap();
    assert!(workspace.create().status.success());
    // この fixture は code-generation へ進むための通り道で、要約確認は試さない
    // （確認のガードは intent_lifecycle が固定する）。2.8.2 と同じ逃げ道で外す。
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .args(args)
            .current_dir(workspace.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", workspace.path())
            .env("PATH", "/usr/bin:/bin")
            .env("AIDLC_SKIP_SUMMARY_CONFIRMATION_GUARD", "1")
            .output()
            .unwrap()
    };
    for _ in 0..33 {
        if workspace
            .state()
            .contains("**Current Stage**: code-generation")
        {
            break;
        }
        if workspace
            .state()
            .contains("**Current Stage**: requirements-analysis")
        {
            for verdict in [false, true] {
                // 本家のレビュー要求は `produces` 全文書の実在を要し（aidlc-log.ts:1997-2001）、
                // 完了は要求時のバイトへ後から加えた `## Review` 付録だけを許す（:2135-2160）。
                workspace.write_requirements_documents(verdict);
                let mut args = vec![
                    "review",
                    "--stage",
                    "requirements-analysis",
                    "--reviewer",
                    "aidlc-product-lead-agent",
                    "--iteration",
                    "1",
                ];
                if verdict {
                    args.extend(["--verdict", "READY"]);
                }
                let reviewed = workspace.log(&args);
                assert!(reviewed.status.success(), "{reviewed:?}");
            }
        }
        if workspace
            .state()
            .contains("**Current Stage**: reverse-engineering")
        {
            workspace.complete_reverse_engineering_pipeline();
        }
        let opened = run(&["report", "--result", "awaiting-approval"]);
        assert!(opened.status.success(), "{opened:?}");
        // 業務拒否は終了 0 のまま stdout に `{"kind":"error"}` を返すので、ここで音を立てる。
        assert!(
            !String::from_utf8_lossy(&opened.stdout).contains("\"kind\":\"error\""),
            "{opened:?}"
        );
        assert!(
            workspace
                .human_prompt(
                    r#"{"session_id":"stage-advance","prompt":"Approve"}"#,
                    false
                )
                .status
                .success()
        );
        let approved = run(&["report", "--result", "approved", "--user-input", "Approve"]);
        assert!(approved.status.success(), "{approved:?}");
        assert!(
            !String::from_utf8_lossy(&approved.stdout).contains("\"kind\":\"error\""),
            "{approved:?}"
        );
    }
    assert!(
        workspace
            .state()
            .contains("**Current Stage**: code-generation"),
        "{}",
        workspace.state()
    );
    workspace
}

#[test]
fn code_generation_publication_records_a_source_floor_and_authority_revision() {
    let workspace = workspace_at_code_generation();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .args(args)
            .current_dir(workspace.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", workspace.path())
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap()
    };
    let output = run(&["next"]);
    assert!(output.status.success(), "{output:?}");
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let marker: serde_json::Value =
        serde_json::from_slice(&fs::read(record.join(".aidlc-active-directive.json")).unwrap())
            .unwrap();
    assert_eq!(
        marker.get("stage").and_then(serde_json::Value::as_str),
        Some("code-generation")
    );
    assert_eq!(
        marker.get("code_generation_authority_revision"),
        marker.get("revision")
    );
    let floor = marker
        .get("code_generation_source_sha256")
        .and_then(serde_json::Value::as_str)
        .unwrap();
    assert_eq!(floor.len(), 64);
    assert!(
        floor
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    );
    fs::write(
        workspace.path().join("new-source.rs"),
        "pub fn changed() {}\n",
    )
    .unwrap();
    assert!(run(&["next"]).status.success());
    let reissued: serde_json::Value =
        serde_json::from_slice(&fs::read(record.join(".aidlc-active-directive.json")).unwrap())
            .unwrap();
    assert_eq!(
        reissued.get("code_generation_source_sha256"),
        marker.get("code_generation_source_sha256"),
        "未承認の再発行では提示開始時のソース基準を保持する"
    );
    assert_eq!(
        reissued
            .get("code_generation_authority_revision")
            .and_then(serde_json::Value::as_u64),
        Some(2)
    );
}

#[test]
fn testing_posture_render_reads_the_projected_contract() {
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    let binary = workspace.path().join("bin/aidlc-testing-posture");
    fs::copy(env!("CARGO_BIN_EXE_aidlc"), &binary).unwrap();
    let output = Command::new(binary)
        .arg("render")
        .current_dir(workspace.path())
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", workspace.path())
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/testing-posture.json"
    ))
    .unwrap();
    let expected = corpus
        .get("observations")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|case| {
            case.get("id").and_then(serde_json::Value::as_str) == Some("empty-bugfix-minimal")
        })
        .unwrap();
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        expected.get("rendered").unwrap().as_str().unwrap()
    );
}

#[test]
fn testing_contract_refreshes_memory_without_workflow_events_and_recovers_atomically() {
    use core_query_interface_adapter::ReadModelDaos;
    use core_query_use_case::orchestration::FindTestingContractUseCase;
    let workspace = Workspace::new();
    fs::write(workspace.path().join("main.ts"), "export const base = 1;\n").unwrap();
    assert!(workspace.create().status.success());
    let binary = workspace.path().join("bin/aidlc-testing-posture");
    fs::copy(env!("CARGO_BIN_EXE_aidlc"), &binary).unwrap();
    let render = || {
        Command::new(&binary)
            .arg("render")
            .current_dir(workspace.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", workspace.path())
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap()
    };
    assert!(render().status.success());
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let cursor = fs::read_to_string(record.join(".aidlc-execution")).unwrap();
    let intent_id = cursor.lines().nth(1).unwrap();
    let store = intents.join(".aidlc-store.sqlite");
    let daos = ReadModelDaos::open(&store).unwrap();
    let query = FindTestingContractUseCase::new(daos.testing_contract());
    let original = query.execute(intent_id).unwrap().unwrap();
    assert_eq!(query.execute("unknown-intent").unwrap(), None);
    let db = rusqlite::Connection::open(&store).unwrap();
    let event_count = || {
        db.query_row("SELECT count(*) FROM journal", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap()
    };
    let count = event_count();
    let state = workspace.state();
    let audit = || {
        fs::read_dir(record.join("audit"))
            .unwrap()
            .map(|entry| fs::read(entry.unwrap().path()).unwrap())
            .collect::<Vec<_>>()
    };
    let original_audit = audit();
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/testing-posture.json"
    ))
    .unwrap();
    let observed = |id: &str| {
        corpus
            .get("observations")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case.get("id").and_then(serde_json::Value::as_str) == Some(id))
            .unwrap()
    };
    let memory = workspace.path().join("aidlc/spaces/default/memory");
    fs::create_dir_all(&memory).unwrap();
    let change = |case: &serde_json::Value| {
        fs::write(
            memory.join("team.md"),
            format!(
                "## Testing Posture\n{}",
                case.get("sections")
                    .unwrap()
                    .get("team")
                    .unwrap()
                    .as_str()
                    .unwrap()
            ),
        )
        .unwrap()
    };
    let tdd = observed("team-explicit-tdd");
    change(tdd);
    assert_eq!(
        query.execute(intent_id).unwrap().unwrap(),
        original,
        "Query単独は規則を読み直さない"
    );
    let changed = render();
    assert!(changed.status.success(), "{changed:?}");
    assert_eq!(
        String::from_utf8(changed.stdout).unwrap(),
        tdd.get("rendered").unwrap().as_str().unwrap()
    );
    let stable = query.execute(intent_id).unwrap().unwrap();
    let next = observed("bare-bold-methodology");
    change(next);
    db.execute_batch("CREATE TRIGGER stop_testing_publication BEFORE INSERT ON read_testing_contract BEGIN SELECT RAISE(ABORT,'testing projection unavailable'); END;").unwrap();
    assert!(!render().status.success(), "参照投影の失敗を成功にしない");
    assert_eq!(
        query.execute(intent_id).unwrap().unwrap(),
        stable,
        "削除後の挿入失敗でも旧行が保持される"
    );
    db.execute_batch("DROP TRIGGER stop_testing_publication")
        .unwrap();
    let recovered = render();
    assert!(recovered.status.success(), "{recovered:?}");
    assert_eq!(
        String::from_utf8(recovered.stdout).unwrap(),
        next.get("rendered").unwrap().as_str().unwrap()
    );
    assert_eq!(event_count(), count);
    assert_eq!(workspace.state(), state);
    assert_eq!(audit(), original_audit);
}

#[test]
fn plan_fingerprint_uses_the_current_projected_plan() {
    let workspace = workspace_at_code_generation();
    let run = |binary: &std::path::Path, args: &[&str]| {
        Command::new(binary)
            .args(args)
            .current_dir(workspace.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", workspace.path())
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap()
    };
    let posture = workspace.path().join("bin/aidlc-testing-posture");
    let contract = run(&posture, &["render"]);
    assert!(contract.status.success(), "{contract:?}");
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let directory = record.join("construction/code-generation");
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("code-generation-plan.md"),
        format!(
            "# Plan\n\n{}\n## Steps\n- [ ] Implement\n",
            String::from_utf8(contract.stdout).unwrap()
        ),
    )
    .unwrap();
    fs::write(
        directory.join("unit-test-instructions.md"),
        "# Instructions\nRun cargo test.\n",
    )
    .unwrap();
    let emitted = run(std::path::Path::new(env!("CARGO_BIN_EXE_aidlc")), &["next"]);
    assert!(emitted.status.success(), "{emitted:?}");
    let fingerprint = run(&posture, &["fingerprint", "--stage-level"]);
    assert!(fingerprint.status.success(), "{fingerprint:?}");
    let fingerprint = String::from_utf8(fingerprint.stdout).unwrap();
    // 2.8.2 の形 — 計画承認節へ写す 2 行のタグ（内容の指紋と、計画を書いたときのソース）。
    let tag = fingerprint.lines().next().unwrap();
    let value = tag.strip_prefix("[Approval Fingerprint]: ").unwrap();
    assert!(value.starts_with("sha256:"));
    assert_eq!(value.len(), 71);
    assert!(
        fingerprint
            .lines()
            .nth(1)
            .unwrap()
            .starts_with("[Planned Source]: ")
    );
    let db = rusqlite::Connection::open(intents.join(".aidlc-store.sqlite")).unwrap();
    let cursor = fs::read_to_string(record.join(".aidlc-execution")).unwrap();
    let execution = cursor.lines().next().unwrap();
    let saved: String = db.query_row("SELECT fingerprint FROM read_plan_fingerprint WHERE execution_id=?1 AND target_id='stage:code-generation'", [execution], |row| row.get(0)).unwrap();
    assert_eq!(value, saved);
    drop(db);
    let relocated_parent = tempfile::tempdir().unwrap();
    let relocated = relocated_parent.path().join("workspace");
    fs::rename(workspace.path(), &relocated).unwrap();
    let copied = Command::new(relocated.join("bin/aidlc-testing-posture"))
        .args(["fingerprint", "--stage-level"])
        .current_dir(&relocated)
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", &relocated)
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    let observed: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/plan-project-binding.json"
    ))
    .unwrap();
    let expected_code = observed
        .get("observations")
        .unwrap()
        .as_array()
        .unwrap()
        .first()
        .unwrap()
        .get("output")
        .unwrap()
        .get("exit_code")
        .unwrap()
        .as_i64()
        .unwrap();
    assert_eq!(copied.status.code().map(i64::from), Some(expected_code));
    assert_eq!(
        String::from_utf8(copied.stdout).unwrap().lines().next(),
        Some(tag),
        "本家と同じく同一依頼の複写では保存済み発行に束縛した指紋を保持する"
    );
}

#[test]
fn an_invalid_utf8_stdin_still_records_an_anonymous_human_turn() {
    use base64::Engine as _;
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/human-invalid-utf8.json"
    ))
    .unwrap();
    let observed = corpus
        .get("observations")
        .unwrap()
        .as_array()
        .unwrap()
        .first()
        .unwrap();
    let input = base64::engine::general_purpose::STANDARD
        .decode(
            observed
                .get("input")
                .unwrap()
                .get("stdin_base64")
                .unwrap()
                .as_str()
                .unwrap(),
        )
        .unwrap();
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let files: Vec<_> = fs::read_dir(record.join("audit"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1);
    let audit_path = files.first().unwrap();
    let before_audit = fs::read(audit_path).unwrap();
    let state = workspace.state();
    let output = workspace.human_bytes(&input, false);
    assert_eq!(
        output.status.code().map(i64::from),
        observed
            .get("output")
            .unwrap()
            .get("exit_code")
            .unwrap()
            .as_i64()
    );
    let store = workspace
        .path()
        .join("aidlc/spaces/default/intents/.aidlc-store.sqlite");
    let database = rusqlite::Connection::open(store).unwrap();
    let count: i64 = database.query_row("SELECT count(*) FROM journal WHERE json_extract(CAST(payload AS TEXT), '$.PromptObserved.session')='' AND json_extract(CAST(payload AS TEXT), '$.PromptObserved.response')=''", [], |row| row.get(0)).unwrap();
    assert_eq!(count, 1, "不正JSONでも既存の匿名HUMAN_TURNを欠落させない");
    assert_eq!(workspace.state(), state);
    let output_bytes = |field: &str| {
        base64::engine::general_purpose::STANDARD
            .decode(
                observed
                    .get("output")
                    .unwrap()
                    .get(field)
                    .unwrap()
                    .as_str()
                    .unwrap(),
            )
            .unwrap()
    };
    assert_eq!(output.stdout, output_bytes("stdout_base64"));
    assert_eq!(output.stderr, output_bytes("stderr_base64"));
    let after_audit = fs::read(audit_path).unwrap();
    let appended = after_audit.strip_prefix(before_audit.as_slice()).unwrap();
    let expected = base64::engine::general_purpose::STANDARD
        .decode(
            observed
                .get("appended_audit_base64")
                .unwrap()
                .as_str()
                .unwrap(),
        )
        .unwrap();
    let timestamps = |bytes: &[u8]| {
        String::from_utf8(bytes.to_vec())
            .unwrap()
            .split_inclusive('\n')
            .map(|line| {
                if line.starts_with("**Timestamp**:") {
                    "**Timestamp**: <timestamp>\n"
                } else {
                    line
                }
            })
            .collect::<String>()
    };
    assert_eq!(timestamps(appended), timestamps(&expected));
}

#[test]
fn regular_next_prepares_and_resolves_shared_invalidation_but_probe_does_not() {
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    let shared = workspace.path().join("aidlc/.aidlc-runtime.sqlite");
    assert!(!shared.exists(), "開始だけでは承認用ストアを作らない");
    let next = |probe: bool| {
        Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .arg("next")
            .current_dir(workspace.path())
            .env("AIDLC_STOP_HOOK_PROBE", if probe { "1" } else { "0" })
            .output()
            .unwrap()
    };
    let probe = next(true);
    assert!(probe.status.success(), "{probe:?}");
    assert!(!shared.exists(), "Stopの確認呼び出しでは共有状態を作らない");
    let published = next(false);
    assert!(published.status.success(), "{published:?}");
    assert!(shared.is_file(), "通常の発行は共有の失効準備も耐久保存する");
    let database = rusqlite::Connection::open(&shared).unwrap();
    assert_eq!(
        database
            .query_row("SELECT count(*) FROM journal", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        database
            .query_row(
                "SELECT count(*) FROM read_plan_operation WHERE status='prepared'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    let id: String = database
        .query_row(
            "SELECT operation_id FROM read_plan_operation WHERE status='applied'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let space = rusqlite::Connection::open(
        workspace
            .path()
            .join("aidlc/spaces/default/intents/.aidlc-store.sqlite"),
    )
    .unwrap();
    let count: i64 = space
        .query_row(
            "SELECT count(*) FROM journal WHERE CAST(payload AS TEXT) LIKE ?1",
            [format!("%{id}%")],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "同じ操作IDで指示発行と失効を対応付ける");
    let again = next(true);
    assert!(again.status.success());
    assert_eq!(again.stdout, published.stdout);
    assert_eq!(
        database
            .query_row("SELECT count(*) FROM journal", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        3
    );
}

#[test]
fn losing_a_used_shared_store_is_refused_without_creating_an_empty_replacement() {
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    let next = || {
        Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .arg("next")
            .current_dir(workspace.path())
            .output()
            .unwrap()
    };
    assert!(next().status.success());
    let shared = workspace.path().join("aidlc/.aidlc-runtime.sqlite");
    assert!(shared.is_file());
    fs::remove_file(&shared).unwrap();
    let output = next();
    let error: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        error.get("kind").and_then(serde_json::Value::as_str),
        Some("error"),
        "{output:?}"
    );
    assert!(!shared.exists());
    assert!(
        error
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap()
            .contains("shared approval store is missing")
    );
}

#[test]
fn stop_and_health_hooks_cannot_recreate_a_lost_approval_store() {
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    assert!(
        Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .arg("next")
            .current_dir(workspace.path())
            .output()
            .unwrap()
            .status
            .success()
    );
    let shared = workspace.path().join("aidlc/.aidlc-runtime.sqlite");
    fs::remove_file(&shared).unwrap();
    let stopped = workspace.stop("{}", &[]);
    assert_eq!(stopped.status.code(), Some(0));
    assert!(
        stopped.stdout.is_empty() && stopped.stderr.is_empty(),
        "{stopped:?}"
    );
    assert!(
        !shared.exists(),
        "Stopのhealth/drop/counterは使用済み承認ストアを作り直さない"
    );
}

#[test]
fn a_used_runtime_cannot_be_reset_by_an_initializing_marker() {
    let workspace = Workspace::new();
    assert!(workspace.create().status.success());
    let next = || {
        Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .arg("next")
            .current_dir(workspace.path())
            .output()
            .unwrap()
    };
    assert!(next().status.success());
    let marker = workspace.path().join("aidlc/.aidlc-runtime.state.json");
    // 初回終了前の古い機械ローカル記録へ戻った、不整合な復旧入力。
    fs::write(&marker, "{\"version\":1,\"phase\":\"initializing\"}\n").unwrap();
    let output = next();
    let error: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        error.get("kind").and_then(serde_json::Value::as_str),
        Some("error"),
        "{output:?}"
    );
    let database =
        rusqlite::Connection::open(workspace.path().join("aidlc/.aidlc-runtime.sqlite")).unwrap();
    assert_eq!(
        database
            .query_row("SELECT count(*) FROM journal", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        3
    );
    assert!(fs::read_to_string(marker).unwrap().contains("initializing"));
}

#[test]
fn a_new_intent_in_either_space_cannot_hide_loss_of_the_used_shared_store() {
    for other_space in [false, true] {
        let workspace = Workspace::new();
        assert!(workspace.create().status.success());
        let next = || {
            Command::new(env!("CARGO_BIN_EXE_aidlc"))
                .arg("next")
                .current_dir(workspace.path())
                .output()
                .unwrap()
        };
        let first: serde_json::Value = serde_json::from_slice(&next().stdout).unwrap();
        assert_eq!(
            first.get("kind").and_then(serde_json::Value::as_str),
            Some("run-stage")
        );
        let shared = workspace.path().join("aidlc/.aidlc-runtime.sqlite");
        fs::remove_file(&shared).unwrap();
        if other_space {
            fs::create_dir_all(workspace.path().join("aidlc/spaces/other/intents")).unwrap();
            fs::write(workspace.path().join("aidlc/active-space"), "other\n").unwrap();
        }
        let created = Command::new(workspace.path().join("bin/aidlc-utility"))
            .args([
                "intent-create",
                "--scope",
                "bugfix",
                "--label",
                "second",
                "--arguments",
                "Another defect",
            ])
            .current_dir(workspace.path())
            .output()
            .unwrap();
        assert!(created.status.success(), "{created:?}");
        assert!(!shared.exists(), "2件目の開始は承認ストアを新規作成しない");
        let output = next();
        let error: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            error.get("kind").and_then(serde_json::Value::as_str),
            Some("error"),
            "other_space={other_space}: {output:?}"
        );
        assert!(
            error
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap()
                .contains("shared approval store is missing")
        );
        assert!(!shared.exists());
    }
}

#[test]
fn interrupted_initialization_recovers_before_any_approval_operation_is_allowed() {
    use core_command_domain::orchestration::PlanApprovalRuntime;
    use core_command_domain::workspace::StorePath;
    use core_command_interface_adapter::orchestration::PlanApprovalRuntimeRepositoryImpl;
    use core_command_use_case::orchestration::PlanApprovalRuntimeRepository;
    for created_saved in [false, true] {
        let workspace = Workspace::new();
        assert!(workspace.create().status.success());
        // 初回作成の途中で停止した機械ローカルのチェックポイントを明示して再現する。
        fs::write(
            workspace.path().join("aidlc/.aidlc-runtime.state.json"),
            "{\"version\":1,\"phase\":\"initializing\"}\n",
        )
        .unwrap();
        let shared = StorePath::for_runtime(&workspace.path().join("aidlc"));
        if created_saved {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let mut repository = PlanApprovalRuntimeRepositoryImpl::open(&shared).unwrap();
                    let (runtime, event) = PlanApprovalRuntime::create(chrono::Utc::now());
                    repository.store(&event, &runtime).await.unwrap();
                });
        }
        let output = Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .arg("next")
            .current_dir(workspace.path())
            .output()
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            value.get("kind").and_then(serde_json::Value::as_str),
            Some("run-stage"),
            "created_saved={created_saved}: {output:?}"
        );
        let marker: serde_json::Value = serde_json::from_slice(
            &fs::read(workspace.path().join("aidlc/.aidlc-runtime.state.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            marker.get("phase").and_then(serde_json::Value::as_str),
            Some("ready")
        );
        let db = rusqlite::Connection::open(shared.as_path()).unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM journal", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            3
        );
    }
}

fn workspace_with_plan_questions() -> (Workspace, String) {
    let workspace = workspace_at_code_generation();
    let posture = workspace.path().join("bin/aidlc-testing-posture");
    let run = |binary: &std::path::Path, args: &[&str]| {
        Command::new(binary)
            .args(args)
            .current_dir(workspace.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", workspace.path())
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap()
    };
    let contract = run(&posture, &["render"]);
    assert!(contract.status.success(), "{contract:?}");
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let directory = record.join("construction/code-generation");
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("code-generation-plan.md"),
        format!(
            "# Plan\n\n{}\n## Steps\n- [ ] Implement\n",
            String::from_utf8(contract.stdout).unwrap()
        ),
    )
    .unwrap();
    fs::write(
        directory.join("unit-test-instructions.md"),
        "# Instructions\nRun cargo test.\n",
    )
    .unwrap();
    let emitted = run(std::path::Path::new(env!("CARGO_BIN_EXE_aidlc")), &["next"]);
    assert!(emitted.status.success(), "{emitted:?}");
    let fingerprint = run(&posture, &["fingerprint", "--stage-level"]);
    assert!(fingerprint.status.success(), "{fingerprint:?}");
    let fingerprint = String::from_utf8(fingerprint.stdout).unwrap();
    let questions = directory.join("code-generation-questions.md");
    // 指紋の出力は計画承認節へそのまま写す 2 行のタグである（2.8.2 の形）。
    assert!(
        fingerprint.starts_with("[Approval Fingerprint]: sha256:"),
        "{fingerprint}"
    );
    assert!(
        fingerprint.contains("\n[Planned Source]: "),
        "{fingerprint}"
    );
    fs::write(
        &questions,
        format!(
            "## Plan Approval\n{}\nA. Approve Plan\nB. Request Changes\n[Answer]:\n",
            fingerprint.trim()
        ),
    )
    .unwrap();
    let relative = questions
        .strip_prefix(workspace.path())
        .unwrap()
        .to_str()
        .unwrap()
        .replace('\\', "/");
    (workspace, relative)
}

#[test]
fn a_plan_approval_decision_persists_its_target_and_projects_the_offered_challenge() {
    let (workspace, questions) = workspace_with_plan_questions();
    let before = workspace.state();
    let decision_args = [
        "decision",
        "--stage",
        "code-generation",
        "--checkpoint",
        "plan-approval",
        "--questions-file",
        &questions,
        "--session",
        "stage1-session",
        "--stage-level",
        "--decision",
        "Approve this exact Code Generation plan?",
        "--options",
        "Approve Plan,Request Changes",
    ];
    let output = workspace.log(&decision_args);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        output.stdout,
        b"{\"emitted\":\"DECISION_RECORDED\",\"stage\":\"code-generation\"}\n"
    );
    assert_eq!(workspace.state(), before);
    let challenge_path = workspace
        .path()
        .join("aidlc/.aidlc-sessions/plan-approval/challenge-stage1-session.json");
    let challenge: serde_json::Value =
        serde_json::from_slice(&fs::read(&challenge_path).unwrap()).unwrap();
    use base64::Engine as _;
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/upstream-a277af21/stage1/cases.json"
    ))
    .unwrap();
    let observed = corpus
        .get("observations")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case.get("id").and_then(serde_json::Value::as_str) == Some("plan/decision"))
        .unwrap();
    let saved = observed
        .get("changed_files")
        .unwrap()
        .as_object()
        .unwrap()
        .iter()
        .find(|(name, _)| name.ends_with("challenge-stage1-session.json"))
        .unwrap()
        .1
        .as_str()
        .unwrap();
    let reference: serde_json::Value = serde_json::from_slice(
        &base64::engine::general_purpose::STANDARD
            .decode(saved)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        challenge.as_object().unwrap().keys().collect::<Vec<_>>(),
        reference.as_object().unwrap().keys().collect::<Vec<_>>()
    );
    assert_eq!(
        challenge
            .get("targetId")
            .and_then(serde_json::Value::as_str),
        Some("stage:code-generation")
    );
    assert_eq!(
        challenge.get("session").and_then(serde_json::Value::as_str),
        Some("stage1-session")
    );
    assert_eq!(
        challenge
            .get("questionsFile")
            .and_then(serde_json::Value::as_str),
        Some(questions.as_str())
    );
    assert_eq!(challenge.get("options"), reference.get("options"));
    let db =
        rusqlite::Connection::open(workspace.path().join("aidlc/.aidlc-runtime.sqlite")).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM journal WHERE CAST(payload AS TEXT) LIKE '%ChallengeIssued%'",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let audit: String = fs::read_dir(record.join("audit"))
        .unwrap()
        .map(|file| fs::read_to_string(file.unwrap().path()).unwrap())
        .collect();
    assert!(audit.contains("**Checkpoint**: Code Generation Plan Approval"));
    assert!(audit.contains(&format!("**Questions File**: {questions}")));
    assert!(audit.contains("**Session**: stage1-session"));
    // 採取コマンドだけが実行する同一入力の外部比較。通常のRust CIは保存済み資料だけを使う。
    if let Ok(destination) = std::env::var("AIDLC_PLAN_DECISION_CAPTURE") {
        let native_challenge = fs::read(&challenge_path).unwrap();
        let source = PathBuf::from(std::env::var("AIDLC_PLAN_DECISION_SOURCE").unwrap());
        let bun = PathBuf::from(std::env::var("AIDLC_PLAN_DECISION_BUN").unwrap());
        let mut inputs = serde_json::Map::new();
        for path in [
            record.join("aidlc-state.md"),
            record.join(".aidlc-active-directive.json"),
            workspace.path().join(&questions),
            record.join("construction/code-generation/code-generation-plan.md"),
            record.join("construction/code-generation/unit-test-instructions.md"),
            workspace.path().join("aidlc/spaces/default/memory/org.md"),
            workspace.path().join("aidlc/spaces/default/memory/team.md"),
            workspace
                .path()
                .join("aidlc/spaces/default/memory/project.md"),
        ] {
            let relative = path
                .strip_prefix(workspace.path())
                .unwrap()
                .to_str()
                .unwrap()
                .replace('\\', "/");
            inputs.insert(
                relative,
                fs::read(path).map_or(serde_json::Value::Null, |bytes| {
                    serde_json::Value::String(
                        base64::engine::general_purpose::STANDARD.encode(bytes),
                    )
                }),
            );
        }
        fs::remove_file(&challenge_path).unwrap();
        assert!(
            !challenge_path.exists(),
            "本家実行前にchallengeが存在しない"
        );
        let upstream = Command::new(&bun)
            .arg(source.join(".claude/tools/aidlc-log.ts"))
            .args(decision_args)
            .current_dir(workspace.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", workspace.path().join("aidlc/.capture-home"))
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", bun.parent().unwrap().display()),
            )
            .env("CLAUDE_PROJECT_DIR", workspace.path())
            .env("AIDLC_PROJECT_DIR", workspace.path())
            .output()
            .unwrap();
        assert!(
            challenge_path.is_file(),
            "本家decisionがchallengeを新規生成する"
        );
        let source_challenge = fs::read(&challenge_path).unwrap();
        let source_module = capture_json::literal(&source.join(".claude/tools/aidlc-lib.ts"));
        let project_literal = capture_json::literal(&workspace.path());
        let probe_code = format!(
            "const m = await import({source_module}); const s=m.workspaceSourceState({project_literal}); console.log(JSON.stringify({{fingerprint:s?.fingerprint ?? null,repos:m.intentRepos({project_literal}),listing:s ? [...s.listing] : null}}));"
        );
        let inspected = Command::new(&bun)
            .args(["--eval", &probe_code])
            .current_dir(workspace.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", workspace.path().join("aidlc/.capture-home"))
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap();
        let inspected_value: serde_json::Value = serde_json::from_slice(&inspected.stdout).unwrap();
        let native_value: serde_json::Value = serde_json::from_slice(&native_challenge).unwrap();
        let source_unchanged =
            inspected_value.get("fingerprint") == native_value.get("sourceFloor");
        let record = capture_json::object([
            (
                "source_inspection",
                capture_json::value(&String::from_utf8_lossy(&inspected.stdout)),
            ),
            (
                "input",
                capture_json::object([
                    ("project_dir", capture_json::value(&workspace.path())),
                    ("argv", capture_json::value(&decision_args)),
                    (
                        "public_files_before_source_base64",
                        capture_json::value(&inputs),
                    ),
                    (
                        "source_files_unchanged_between_commands",
                        capture_json::value(&source_unchanged),
                    ),
                ]),
            ),
            (
                "native",
                capture_json::file_output(&output, "challenge_base64", &native_challenge),
            ),
            (
                "upstream",
                capture_json::file_output(&upstream, "challenge_base64", &source_challenge),
            ),
        ]);
        fs::write(destination, capture_json::pretty(&record)).unwrap();
        assert!(upstream.status.success(), "{upstream:?}");
        assert!(
            source_unchanged,
            "比較中のソース指紋は発行時の実測値と一致する"
        );
        assert_eq!(output.stdout, upstream.stdout);
        assert_eq!(
            native_challenge, source_challenge,
            "同じ入力に対する全フィールド/全バイト。導出ハッシュも正規化しない"
        );
    }
}

#[test]
fn a_real_human_turn_is_delivered_to_the_exact_plan_offer_and_projected() {
    let (workspace, questions) = workspace_with_plan_questions();
    let asked = workspace.log(&[
        "decision",
        "--stage",
        "code-generation",
        "--checkpoint",
        "plan-approval",
        "--questions-file",
        &questions,
        "--session",
        "stage1-session",
        "--stage-level",
        "--decision",
        "Approve this exact Code Generation plan?",
        "--options",
        "Approve Plan,Request Changes",
    ]);
    assert!(asked.status.success(), "{asked:?}");
    let directory = workspace.path().join("aidlc/.aidlc-sessions/plan-approval");
    let challenge: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.join("challenge-stage1-session.json")).unwrap())
            .unwrap();
    let before = workspace.state();
    assert!(
        !directory.join("response-stage1-session.json").exists(),
        "native実行前にもresponseがない"
    );
    let observed = workspace.human_prompt(
        r#"{"session_id":"stage1-session","prompt":"Approve Plan"}"#,
        false,
    );
    assert!(observed.status.success());
    assert!(observed.stdout.is_empty() && observed.stderr.is_empty());
    let response = directory.join("response-stage1-session.json");
    assert!(response.is_file(), "実際の応答を共有承認へ保存・投影する");
    let body = fs::read(&response).unwrap();
    let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(response.get("challengeId"), challenge.get("challengeId"));
    assert_eq!(
        response.get("session").and_then(serde_json::Value::as_str),
        Some("stage1-session")
    );
    assert_eq!(
        response.get("choice").and_then(serde_json::Value::as_str),
        Some("Approve Plan")
    );
    assert_eq!(
        response
            .get("responseSha256")
            .and_then(serde_json::Value::as_str),
        Some("06a26f18664ec83f69387fc4490db709001e649dd1e336ea46d7d67701f92af9")
    );
    assert_eq!(workspace.state(), before);
    let root =
        rusqlite::Connection::open(workspace.path().join("aidlc/.aidlc-runtime.sqlite")).unwrap();
    assert_eq!(
        root.query_row(
            "SELECT count(*) FROM journal WHERE CAST(payload AS TEXT) LIKE '%ResponsePrepared%'",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        root.query_row(
            "SELECT count(*) FROM journal WHERE CAST(payload AS TEXT) LIKE '%ResponseObserved%'",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    let source = rusqlite::Connection::open(
        workspace
            .path()
            .join("aidlc/spaces/default/intents/.aidlc-store.sqlite"),
    )
    .unwrap();
    assert_eq!(source.query_row("SELECT count(*) FROM journal WHERE CAST(payload AS TEXT) LIKE '%approval_observation_id%'", [], |row| row.get::<_,i64>(0)).unwrap(), 1);
    assert!(
        workspace
            .human_prompt(r#"{"session_id":"another-session","prompt":"2"}"#, false)
            .status
            .success()
    );
    assert_eq!(
        fs::read(directory.join("response-stage1-session.json")).unwrap(),
        body
    );
    if let Ok(destination) = std::env::var("AIDLC_PLAN_HUMAN_CAPTURE") {
        use std::io::Write as _;
        let source = PathBuf::from(std::env::var("AIDLC_PLAN_DECISION_SOURCE").unwrap());
        let bun = PathBuf::from(std::env::var("AIDLC_PLAN_DECISION_BUN").unwrap());
        let input = r#"{"session_id":"stage1-session","prompt":"Approve Plan"}"#;
        let response_path = directory.join("response-stage1-session.json");
        fs::remove_file(&response_path).unwrap();
        assert!(!response_path.exists(), "本家実行前にresponseが存在しない");
        let mut process = Command::new(&bun)
            .arg(source.join(".claude/hooks/aidlc-record-human-turn.ts"))
            .current_dir(workspace.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", workspace.path().join("aidlc/.capture-home"))
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", bun.parent().unwrap().display()),
            )
            .env("CLAUDE_PROJECT_DIR", workspace.path())
            .env("AIDLC_PROJECT_DIR", workspace.path())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        process
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let upstream = process.wait_with_output().unwrap();
        let after = fs::read(&response_path).ok();
        let captured = capture_json::object([
            (
                "input",
                capture_json::object([
                    ("project_dir", capture_json::value(&workspace.path())),
                    ("stdin", capture_json::value(&input)),
                    ("challenge", capture_json::value(&challenge)),
                ]),
            ),
            (
                "native",
                capture_json::file_output(&observed, "response_base64", &body),
            ),
            (
                "upstream",
                capture_json::file_output(
                    &upstream,
                    "response_base64",
                    after.as_deref().unwrap_or_default(),
                ),
            ),
        ]);
        fs::write(destination, capture_json::pretty(&captured)).unwrap();
        assert!(upstream.status.success(), "{upstream:?}");
        assert!(after.is_some(), "本家実行でresponseが新規生成される");
        assert_eq!(observed.stdout, upstream.stdout);
        assert_eq!(observed.stderr, upstream.stderr);
        assert_eq!(
            Some(body),
            after,
            "同じ提示・実際の応答に対する本家の全バイト"
        );
    }
}

#[test]
fn a_saved_human_turn_is_not_duplicated_when_shared_response_delivery_restarts() {
    let (workspace, questions) = workspace_with_plan_questions();
    assert!(
        workspace
            .log(&[
                "decision",
                "--stage",
                "code-generation",
                "--checkpoint",
                "plan-approval",
                "--questions-file",
                &questions,
                "--session",
                "stage1-session",
                "--stage-level",
                "--decision",
                "Approve this exact Code Generation plan?",
                "--options",
                "Approve Plan,Request Changes"
            ])
            .status
            .success()
    );
    let root =
        rusqlite::Connection::open(workspace.path().join("aidlc/.aidlc-runtime.sqlite")).unwrap();
    root.execute_batch("CREATE TRIGGER fail_response_delivery BEFORE INSERT ON journal WHEN json_extract(CAST(NEW.payload AS TEXT),'$.payload.type')='ResponseObserved' BEGIN SELECT RAISE(ABORT, 'injected response delivery failure'); END;").unwrap();
    let observed = workspace.human_prompt(r#"{"session_id":"stage1-session","prompt":"1"}"#, false);
    assert!(observed.status.success(), "フック自体は本家どおりfail-open");
    assert_eq!(
        root.query_row(
            "SELECT count(*) FROM read_plan_operation WHERE status='prepared' AND kind='response'",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    let source = rusqlite::Connection::open(
        workspace
            .path()
            .join("aidlc/spaces/default/intents/.aidlc-store.sqlite"),
    )
    .unwrap();
    assert_eq!(source.query_row("SELECT count(*) FROM journal WHERE CAST(payload AS TEXT) LIKE '%approval_observation_id%'", [], |row| row.get::<_,i64>(0)).unwrap(), 1);
    let response = workspace
        .path()
        .join("aidlc/.aidlc-sessions/plan-approval/response-stage1-session.json");
    assert!(!response.exists(), "準備だけでは受領済みにしない");
    root.execute_batch("DROP TRIGGER fail_response_delivery")
        .unwrap();
    assert!(
        workspace
            .human_prompt(r#"{"session_id":"other","prompt":"unrelated"}"#, false)
            .status
            .success()
    );
    let received: serde_json::Value =
        serde_json::from_slice(&fs::read(&response).unwrap()).unwrap();
    assert_eq!(
        received.get("choice").and_then(serde_json::Value::as_str),
        Some("Approve Plan")
    );
    assert_eq!(source.query_row("SELECT count(*) FROM journal WHERE CAST(payload AS TEXT) LIKE '%approval_observation_id%'", [], |row| row.get::<_,i64>(0)).unwrap(), 1);
    assert_eq!(
        root.query_row(
            "SELECT count(*) FROM journal WHERE CAST(payload AS TEXT) LIKE '%ResponseObserved%'",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        root.query_row(
            "SELECT count(*) FROM read_plan_operation WHERE status='prepared'",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}

#[test]
fn recovered_old_responses_do_not_become_answers_to_reissued_questions() {
    for changed in [false, true] {
        let (workspace, questions) = workspace_with_plan_questions();
        let ask = || {
            workspace.log(&[
                "decision",
                "--stage",
                "code-generation",
                "--checkpoint",
                "plan-approval",
                "--questions-file",
                &questions,
                "--session",
                "stage1-session",
                "--stage-level",
                "--decision",
                "Approve this exact Code Generation plan?",
                "--options",
                "Approve Plan,Request Changes",
            ])
        };
        assert!(ask().status.success());
        let directory = workspace.path().join("aidlc/.aidlc-sessions/plan-approval");
        let old: serde_json::Value = serde_json::from_slice(
            &fs::read(directory.join("challenge-stage1-session.json")).unwrap(),
        )
        .unwrap();
        let root = rusqlite::Connection::open(workspace.path().join("aidlc/.aidlc-runtime.sqlite"))
            .unwrap();
        root.execute_batch("CREATE TRIGGER fail_response_delivery BEFORE INSERT ON journal WHEN json_extract(CAST(NEW.payload AS TEXT),'$.payload.type')='ResponseObserved' BEGIN SELECT RAISE(ABORT, 'injected response delivery failure'); END;").unwrap();
        workspace.human_prompt(r#"{"session_id":"stage1-session","prompt":"1"}"#, false);
        if changed {
            let path = workspace.path().join(&questions);
            let body = fs::read_to_string(&path).unwrap();
            fs::write(path, format!("{body}\nAdditional question context.\n")).unwrap();
        }
        root.execute_batch("DROP TRIGGER fail_response_delivery")
            .unwrap();
        let offered = ask();
        assert!(offered.status.success(), "{offered:?}");
        let new: serde_json::Value = serde_json::from_slice(
            &fs::read(directory.join("challenge-stage1-session.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(old.get("challengeId") != new.get("challengeId"), changed);
        assert!(
            !directory.join("response-stage1-session.json").exists(),
            "changed={changed}: 公開challengeIdが同じでも、前の発行回の応答は使わない"
        );
        assert_eq!(root.query_row("SELECT count(*) FROM journal WHERE CAST(payload AS TEXT) LIKE '%ResponsePrepared%'", [], |row| row.get::<_,i64>(0)).unwrap(), 1);
        assert_eq!(root.query_row("SELECT count(*) FROM journal WHERE CAST(payload AS TEXT) LIKE '%ResponseObserved%'", [], |row| row.get::<_,i64>(0)).unwrap(), 1);
    }
}

#[test]
fn recovery_from_another_intent_publishes_the_original_human_audit() {
    let (workspace, questions) = workspace_with_plan_questions();
    assert!(
        workspace
            .log(&[
                "decision",
                "--stage",
                "code-generation",
                "--checkpoint",
                "plan-approval",
                "--questions-file",
                &questions,
                "--session",
                "stage1-session",
                "--stage-level",
                "--decision",
                "Approve this exact Code Generation plan?",
                "--options",
                "Approve Plan,Request Changes"
            ])
            .status
            .success()
    );
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let first = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let audit = || {
        fs::read_dir(first.join("audit"))
            .unwrap()
            .map(|entry| fs::read_to_string(entry.unwrap().path()).unwrap())
            .collect::<String>()
    };
    let before = audit();
    let source = rusqlite::Connection::open(intents.join(".aidlc-store.sqlite")).unwrap();
    source.execute_batch("CREATE TRIGGER fail_source_observation BEFORE INSERT ON journal WHEN CAST(NEW.payload AS TEXT) LIKE '%PromptObserved%' BEGIN SELECT RAISE(ABORT, 'injected source observation failure'); END;").unwrap();
    workspace.human_prompt(r#"{"session_id":"stage1-session","prompt":"1"}"#, false);
    assert_eq!(audit(), before);
    let created = Command::new(workspace.path().join("bin/aidlc-utility"))
        .args([
            "intent-create",
            "--scope",
            "bugfix",
            "--label",
            "second",
            "--arguments",
            "Another defect",
        ])
        .current_dir(workspace.path())
        .output()
        .unwrap();
    assert!(created.status.success(), "{created:?}");
    let cursor = fs::read_to_string(intents.join("active-intent")).unwrap();
    source
        .execute_batch("DROP TRIGGER fail_source_observation")
        .unwrap();
    workspace.human_prompt(
        r#"{"session_id":"other-session","prompt":"continue"}"#,
        false,
    );
    assert_eq!(
        fs::read_to_string(intents.join("active-intent")).unwrap(),
        cursor
    );
    assert_eq!(
        audit().matches("**Session**: stage1-session").count(),
        before.matches("**Session**: stage1-session").count() + 1,
        "AのイベントをBから回復しても、Aの監査まで反映する"
    );
}

#[test]
fn a_plan_answer_certifies_the_actual_choice_and_returns_its_projected_result() {
    let (workspace, questions) = workspace_with_plan_questions();
    let asked = workspace.log(&[
        "decision",
        "--stage",
        "code-generation",
        "--checkpoint",
        "plan-approval",
        "--questions-file",
        &questions,
        "--session",
        "stage1-session",
        "--stage-level",
        "--decision",
        "Approve this exact Code Generation plan?",
        "--options",
        "Approve Plan,Request Changes",
    ]);
    assert!(asked.status.success(), "{asked:?}");
    let observed = workspace.human_prompt(r#"{"session_id":"stage1-session","prompt":"1"}"#, false);
    assert!(observed.status.success(), "{observed:?}");
    let questions_path = workspace.path().join(&questions);
    let body = fs::read_to_string(&questions_path)
        .unwrap()
        .replace("[Answer]:\n", "[Answer]: Approve Plan\n");
    fs::write(&questions_path, &body).unwrap();
    let runtime_dir = workspace.path().join("aidlc/.aidlc-sessions/plan-approval");
    let challenge_before = fs::read(runtime_dir.join("challenge-stage1-session.json")).unwrap();
    let response_before = fs::read(runtime_dir.join("response-stage1-session.json")).unwrap();
    let before = workspace.state();
    let output = workspace.log(&[
        "answer",
        "--stage",
        "code-generation",
        "--checkpoint",
        "plan-approval",
        "--questions-file",
        &questions,
        "--session",
        "stage1-session",
        "--stage-level",
        "--details",
        "Approve Plan",
    ]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout,b"{\"emitted\":\"PLAN_APPROVAL_RECORDED\",\"checkpoint\":\"plan-approval\",\"stage\":\"code-generation\"}\n");
    assert_eq!(workspace.state(), before);
    let directory = workspace.path().join("aidlc/.aidlc-sessions/plan-approval");
    let receipts: Vec<_> = fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|file| {
            file.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("receipt-")
        })
        .collect();
    assert_eq!(receipts.len(), 1);
    let receipt: serde_json::Value =
        serde_json::from_slice(&fs::read(receipts.first().unwrap()).unwrap()).unwrap();
    assert_eq!(
        receipt.get("status").and_then(serde_json::Value::as_str),
        Some("approved")
    );
    assert_eq!(
        receipt.get("choice").and_then(serde_json::Value::as_str),
        Some("Approve Plan")
    );
    assert_eq!(
        receipt
            .get("questionsFile")
            .and_then(serde_json::Value::as_str),
        Some(questions.as_str())
    );
    assert_eq!(
        receipt
            .get("questionsSha256")
            .and_then(serde_json::Value::as_str),
        Some(core_infrastructure::hash::sha256_hex(body.as_bytes()).as_str())
    );
    assert!(!directory.join("challenge-stage1-session.json").exists());
    assert!(!directory.join("response-stage1-session.json").exists());
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let audit: String = fs::read_dir(record.join("audit"))
        .unwrap()
        .map(|entry| fs::read_to_string(entry.unwrap().path()).unwrap())
        .collect();
    assert_eq!(
        audit.matches("**Event**: PLAN_APPROVAL_RECORDED").count(),
        1
    );
    let db =
        rusqlite::Connection::open(workspace.path().join("aidlc/.aidlc-runtime.sqlite")).unwrap();
    assert_eq!(db.query_row("SELECT count(*) FROM read_plan_answer WHERE status='recorded' AND emitted='PLAN_APPROVAL_RECORDED'",[],|row|row.get::<_,i64>(0)).unwrap(),1);
    if let Ok(destination) = std::env::var("AIDLC_PLAN_RECEIPT_CAPTURE") {
        use base64::Engine as _;
        let native_receipt = fs::read(receipts.first().unwrap()).unwrap();
        let source = PathBuf::from(std::env::var("AIDLC_PLAN_DECISION_SOURCE").unwrap());
        let bun = PathBuf::from(std::env::var("AIDLC_PLAN_DECISION_BUN").unwrap());
        // Sourceへ同じ回答直前の提示/応答を渡す。nativeの保存正本は比較に使用しない。
        fs::write(
            runtime_dir.join("challenge-stage1-session.json"),
            &challenge_before,
        )
        .unwrap();
        fs::write(
            runtime_dir.join("response-stage1-session.json"),
            &response_before,
        )
        .unwrap();
        let args = [
            "answer",
            "--stage",
            "code-generation",
            "--checkpoint",
            "plan-approval",
            "--questions-file",
            &questions,
            "--session",
            "stage1-session",
            "--stage-level",
            "--details",
            "Approve Plan",
        ];
        fs::remove_file(receipts.first().unwrap()).unwrap();
        assert!(
            !receipts.first().unwrap().exists(),
            "本家実行前に出力receiptが存在しない"
        );
        let upstream = Command::new(&bun)
            .arg(source.join(".claude/tools/aidlc-log.ts"))
            .args(args)
            .current_dir(workspace.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", workspace.path().join("aidlc/.capture-home"))
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", bun.parent().unwrap().display()),
            )
            .env("CLAUDE_PROJECT_DIR", workspace.path())
            .env("AIDLC_PROJECT_DIR", workspace.path())
            .output()
            .unwrap();
        assert!(
            receipts.first().unwrap().is_file(),
            "本家answerがreceiptを新規生成する"
        );
        let source_receipt = fs::read(receipts.first().unwrap()).unwrap();
        let input_files = std::collections::BTreeMap::from([
            (
                "state",
                base64::engine::general_purpose::STANDARD.encode(&before),
            ),
            (
                "questions",
                base64::engine::general_purpose::STANDARD.encode(&body),
            ),
            (
                "plan",
                base64::engine::general_purpose::STANDARD.encode(
                    fs::read(questions_path.with_file_name("code-generation-plan.md")).unwrap(),
                ),
            ),
            (
                "instructions",
                base64::engine::general_purpose::STANDARD.encode(
                    fs::read(questions_path.with_file_name("unit-test-instructions.md")).unwrap(),
                ),
            ),
            (
                "challenge",
                base64::engine::general_purpose::STANDARD.encode(&challenge_before),
            ),
            (
                "response",
                base64::engine::general_purpose::STANDARD.encode(&response_before),
            ),
        ]);
        let captured = capture_json::object([
            (
                "input",
                capture_json::object([
                    ("project_dir", capture_json::value(&workspace.path())),
                    ("argv", capture_json::value(&args)),
                    ("public_files_base64", capture_json::value(&input_files)),
                ]),
            ),
            (
                "native",
                capture_json::file_output(&output, "receipt_base64", &native_receipt),
            ),
            (
                "upstream",
                capture_json::file_output(&upstream, "receipt_base64", &source_receipt),
            ),
        ]);
        fs::write(destination, capture_json::pretty(&captured)).unwrap();
        assert!(upstream.status.success(), "{upstream:?}");
        assert_eq!(output.stdout, upstream.stdout);
        assert_eq!(output.stderr, upstream.stderr);
        assert_eq!(
            native_receipt, source_receipt,
            "同じ回答文書・提示・真正応答に対する受領全バイト"
        );
        if let Ok(readiness) = std::env::var("AIDLC_PLAN_READINESS_CAPTURE") {
            let helper = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../../scripts/goldens/observe-plan-readiness.ts");
            let observed = Command::new(&bun)
                .arg(helper)
                .arg(&source)
                .arg(workspace.path())
                .arg(&questions)
                .arg(receipts.first().unwrap())
                .arg(readiness)
                .current_dir(workspace.path())
                .env_clear()
                .envs(coverage_profile_env())
                .env("HOME", workspace.path().join("aidlc/.capture-home"))
                .env(
                    "PATH",
                    format!("{}:/usr/bin:/bin", bun.parent().unwrap().display()),
                )
                .env("AIDLC_PROJECT_DIR", workspace.path())
                .output()
                .unwrap();
            assert!(observed.status.success(), "{observed:?}");
        }
    }
}

#[test]
fn a_saved_receipt_recovers_its_original_audit_or_revokes_changed_source() {
    for source_changes in [false, true] {
        let (workspace, questions) = workspace_with_plan_questions();
        let asked = workspace.log(&[
            "decision",
            "--stage",
            "code-generation",
            "--checkpoint",
            "plan-approval",
            "--questions-file",
            &questions,
            "--session",
            "stage1-session",
            "--stage-level",
            "--decision",
            "Approve this exact Code Generation plan?",
            "--options",
            "Approve Plan,Request Changes",
        ]);
        assert!(asked.status.success(), "{asked:?}");
        assert!(
            workspace
                .human_prompt(r#"{"session_id":"stage1-session","prompt":"1"}"#, false)
                .status
                .success()
        );
        let file = workspace.path().join(&questions);
        fs::write(
            &file,
            fs::read_to_string(&file)
                .unwrap()
                .replace("[Answer]:\n", "[Answer]: Approve Plan\n"),
        )
        .unwrap();
        let source = rusqlite::Connection::open(
            workspace
                .path()
                .join("aidlc/spaces/default/intents/.aidlc-store.sqlite"),
        )
        .unwrap();
        source.execute_batch("CREATE TRIGGER fail_plan_answer BEFORE INSERT ON journal WHEN CAST(NEW.payload AS TEXT) LIKE '%PlanAnswerLogged%' BEGIN SELECT RAISE(ABORT,'injected plan answer failure'); END;").unwrap();
        let output = workspace.log(&[
            "answer",
            "--stage",
            "code-generation",
            "--checkpoint",
            "plan-approval",
            "--questions-file",
            &questions,
            "--session",
            "stage1-session",
            "--stage-level",
            "--details",
            "Approve Plan",
        ]);
        assert!(
            !output.status.success(),
            "元実行の保存失敗をCLI成功にしない"
        );
        assert!(output.stdout.is_empty());
        let root = rusqlite::Connection::open(workspace.path().join("aidlc/.aidlc-runtime.sqlite"))
            .unwrap();
        let (id, status): (String, String) = root
            .query_row(
                "SELECT operation_id,status FROM read_plan_answer",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, "pending");
        let directory = workspace.path().join("aidlc/.aidlc-sessions/plan-approval");
        let receipts = || {
            fs::read_dir(&directory)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| {
                    path.file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .starts_with("receipt-")
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(receipts().len(), 1, "共有側の受領公開は保存済み");
        assert_eq!(source.query_row("SELECT count(*) FROM journal WHERE CAST(payload AS TEXT) LIKE '%PlanAnswerLogged%'",[],|row|row.get::<_,i64>(0)).unwrap(),0);
        source
            .execute_batch("DROP TRIGGER fail_plan_answer")
            .unwrap();
        if source_changes {
            fs::write(
                workspace.path().join("changed-source.rs"),
                "fn changed() {}\n",
            )
            .unwrap();
        }
        assert!(
            workspace
                .human_prompt(r#"{"session_id":"other","prompt":"unrelated"}"#, false)
                .status
                .success()
        );
        let daos = core_query_interface_adapter::ReadModelDaos::open(
            &workspace.path().join("aidlc/.aidlc-runtime.sqlite"),
        )
        .unwrap();
        let query = core_query_use_case::orchestration::PlanAnswerUseCase::new(daos.plan_answer());
        let result = query.execute(&id).unwrap().unwrap();
        assert_eq!(
            result.status(),
            if source_changes {
                "aborted"
            } else {
                "recorded"
            }
        );
        assert_eq!(receipts().len(), usize::from(!source_changes));
        assert_eq!(
            directory.join("challenge-stage1-session.json").exists(),
            source_changes
        );
        assert_eq!(
            directory.join("response-stage1-session.json").exists(),
            source_changes
        );
        let count=source.query_row("SELECT count(*) FROM journal WHERE CAST(payload AS TEXT) LIKE '%PlanAnswerLogged%'",[],|row|row.get::<_,i64>(0)).unwrap();
        assert_eq!(count, i64::from(!source_changes));
        assert!(
            workspace
                .human_prompt(r#"{"session_id":"other","prompt":"again"}"#, false)
                .status
                .success()
        );
        assert_eq!(source.query_row("SELECT count(*) FROM journal WHERE CAST(payload AS TEXT) LIKE '%PlanAnswerLogged%'",[],|row|row.get::<_,i64>(0)).unwrap(),count);
        let intents = workspace.path().join("aidlc/spaces/default/intents");
        let record = intents.join(
            fs::read_to_string(intents.join("active-intent"))
                .unwrap()
                .trim(),
        );
        let audit: String = fs::read_dir(record.join("audit"))
            .unwrap()
            .map(|entry| fs::read_to_string(entry.unwrap().path()).unwrap())
            .collect();
        assert_eq!(
            audit.matches("**Event**: PLAN_APPROVAL_RECORDED").count(),
            usize::from(!source_changes)
        );
    }
}

#[path = "../../../../tests/support/capture_json.rs"]
mod capture_json;

fn workspace_with_approved_plan() -> Workspace {
    let (workspace, questions) = workspace_with_plan_questions();
    let asked = workspace.log(&[
        "decision",
        "--stage",
        "code-generation",
        "--checkpoint",
        "plan-approval",
        "--questions-file",
        &questions,
        "--session",
        "stage1-session",
        "--stage-level",
        "--decision",
        "Approve this exact Code Generation plan?",
        "--options",
        "Approve Plan,Request Changes",
    ]);
    assert!(asked.status.success(), "{asked:?}");
    assert!(
        workspace
            .human_prompt(r#"{"session_id":"stage1-session","prompt":"1"}"#, false)
            .status
            .success()
    );
    let file = workspace.path().join(&questions);
    fs::write(
        &file,
        fs::read_to_string(&file)
            .unwrap()
            .replace("[Answer]:\n", "[Answer]: Approve Plan\n"),
    )
    .unwrap();
    let answered = workspace.log(&[
        "answer",
        "--stage",
        "code-generation",
        "--checkpoint",
        "plan-approval",
        "--questions-file",
        &questions,
        "--session",
        "stage1-session",
        "--stage-level",
        "--details",
        "Approve Plan",
    ]);
    assert!(answered.status.success(), "{answered:?}");
    workspace
}

/// `brief` は読取専用なので、承認済みワークスペース内に新規ファイルを作らない形で起動する。
fn worker_brief(workspace: &Workspace) -> std::process::Output {
    Command::new(workspace.path().join("bin/aidlc-testing-posture"))
        .args(["brief", "--stage-level"])
        .current_dir(workspace.path())
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", workspace.path().join("aidlc/.capture-home"))
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap()
}

/// 承認済みの計画があれば、ブリーフは契約の指紋と承認済みの 2 文書を運ぶ。
///
/// 投影 → SQL → DAO → ブリーフ組立の終端まで通ることを外から観測する。途中のどこかが
/// 固定値を返す実装なら、承認済みの 2 文書の本文はここに現れない。
#[test]
fn a_worker_brief_for_an_approved_plan_carries_the_contract_hash_and_both_documents() {
    let workspace = workspace_with_approved_plan();
    let before = workspace.state();
    let output = worker_brief(&workspace);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let mut lines = stdout.lines();
    assert_eq!(lines.next(), Some("AIDLC-STAGE: code-generation"));
    let hash = lines
        .next()
        .and_then(|line| line.strip_prefix("AIDLC-TESTING-CONTRACT: "))
        .unwrap_or_default();
    assert!(!hash.is_empty(), "契約の指紋の行が無い — {stdout}");
    for section in [
        "## Approved plan",
        "## Approved unit-test instructions",
        // 承認済みの 2 文書の本文そのもの。
        "## Steps\n- [ ] Implement",
        "Run cargo test.",
    ] {
        assert!(stdout.contains(section), "{section} が無い — {stdout}");
    }
    assert_eq!(
        workspace.state(),
        before,
        "読取専用のブリーフが状態ファイルを動かした"
    );
}

/// 承認後に計画の末尾へ `## Review` 付録を足しても、手順の印を書き換えても、それは作業者へ
/// 届かない（2.8.2 `workerBrief` が渡すのは `projectPlanApprovalContent(plan)`）。
///
/// 指紋は付録と印を見ないので、どちらの改変でも承認は生きたままブリーフが組まれる。そのとき
/// 渡すのが原文なら、付録へ紛れ込ませた未承認の手順がそのまま作業として届いてしまう。
#[test]
fn a_worker_brief_never_delivers_a_review_appendix_or_progress_marks_added_after_approval() {
    let workspace = workspace_with_approved_plan();
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let plan = intents
        .join(
            fs::read_to_string(intents.join("active-intent"))
                .unwrap()
                .trim(),
        )
        .join("construction/code-generation/code-generation-plan.md");
    let approved = fs::read_to_string(&plan).unwrap();
    fs::write(
        &plan,
        format!(
            "{}\n## Review\n\n- [ ] Step 9: unapproved work\n",
            approved.replace("- [ ] Implement", "- [x] Implement")
        ),
    )
    .unwrap();
    let output = worker_brief(&workspace);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("## Steps\n- [ ] Implement"),
        "承認した手順が未着手の形で届いていない — {stdout}"
    );
    for smuggled in ["Step 9", "## Review", "- [x] Implement"] {
        assert!(
            !stdout.contains(smuggled),
            "承認後の改変 {smuggled} が作業者へ届いた — {stdout}"
        );
    }
}

/// plan-approval-guard を PreToolUse の封筒で起動する（終了コード 2 が拒否）。
fn plan_guard(
    workspace: &Workspace,
    tool: &str,
    tool_input: &serde_json::Value,
) -> std::process::Output {
    use std::io::Write as _;
    let mut input = serde_json::Map::new();
    input.insert("session_id".into(), "guard".into());
    input.insert("hook_event_name".into(), "PreToolUse".into());
    input.insert(
        "cwd".into(),
        workspace.path().to_string_lossy().into_owned().into(),
    );
    input.insert("tool_name".into(), tool.into());
    input.insert("tool_input".into(), tool_input.clone());
    let mut child = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .args(["engine", "hook", "plan-approval-guard"])
        .current_dir(workspace.path())
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", workspace.path().join("aidlc/.capture-home"))
        .env("PATH", "/usr/bin:/bin")
        .env("CLAUDE_PROJECT_DIR", workspace.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(serde_json::Value::Object(input).to_string().as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

/// 計画承認の後、承認済みの指示書での開発者派遣とソース書込は通り、生成開始の後は
/// ソースが変わっても承認が失効しない。対象の印の無い派遣は止める。
#[test]
fn the_plan_approval_guard_admits_generation_only_after_approval() {
    let workspace = workspace_with_approved_plan();
    let brief = String::from_utf8(worker_brief(&workspace).stdout).unwrap();
    let source = workspace.path().join("src.rs");
    let task = |prompt: &str| {
        serde_json::Value::Object(serde_json::Map::from_iter([
            ("subagent_type".to_string(), "aidlc-developer-agent".into()),
            ("description".to_string(), "generate".into()),
            ("prompt".to_string(), prompt.into()),
        ]))
    };
    let write = serde_json::Value::Object(serde_json::Map::from_iter([
        (
            "file_path".to_string(),
            source.to_string_lossy().into_owned().into(),
        ),
        ("content".to_string(), "fn main() {}".into()),
    ]));
    let unmarked = plan_guard(&workspace, "Task", &task("implement it"));
    assert_eq!(unmarked.status.code(), Some(2), "{unmarked:?}");
    assert!(
        String::from_utf8_lossy(&unmarked.stderr).contains("the brief does not name it"),
        "{unmarked:?}"
    );
    let dispatched = plan_guard(&workspace, "Task", &task(&brief));
    assert_eq!(dispatched.status.code(), Some(0), "{dispatched:?}");
    let written = plan_guard(&workspace, "Write", &write);
    assert_eq!(written.status.code(), Some(0), "{written:?}");
    fs::write(&source, "fn main() {}\n").unwrap();
    let rewritten = plan_guard(&workspace, "Write", &write);
    assert_eq!(
        rewritten.status.code(),
        Some(0),
        "生成開始の後はソースの変化で失効しない: {rewritten:?}"
    );
}

/// 計画承認の前は、記録ディレクトリの外への書込と開発者派遣を止め、記録の中は通す。
#[test]
fn the_plan_approval_guard_blocks_generation_before_approval() {
    let workspace = workspace_at_code_generation();
    let issued = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .arg("next")
        .current_dir(workspace.path())
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", workspace.path().join("aidlc/.capture-home"))
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    assert!(issued.status.success(), "{issued:?}");
    let write = |path: std::path::PathBuf| {
        serde_json::Value::Object(serde_json::Map::from_iter([
            (
                "file_path".to_string(),
                path.to_string_lossy().into_owned().into(),
            ),
            ("content".to_string(), "x".into()),
        ]))
    };
    let outside = plan_guard(&workspace, "Write", &write(workspace.path().join("src.rs")));
    assert_eq!(outside.status.code(), Some(2), "{outside:?}");
    assert!(
        String::from_utf8_lossy(&outside.stderr).contains("cannot modify workspace path"),
        "{outside:?}"
    );
    let plan = workspace
        .path()
        .join("aidlc/spaces/default/intents")
        .join(
            fs::read_to_string(
                workspace
                    .path()
                    .join("aidlc/spaces/default/intents/active-intent"),
            )
            .unwrap()
            .trim(),
        )
        .join("construction/code-generation/code-generation-plan.md");
    let inside = plan_guard(&workspace, "Write", &write(plan));
    assert_eq!(inside.status.code(), Some(0), "{inside:?}");
    let shell = serde_json::Value::Object(serde_json::Map::from_iter([(
        "command".to_string(),
        "cargo test".into(),
    )]));
    let read_only = plan_guard(&workspace, "Bash", &shell);
    assert_eq!(read_only.status.code(), Some(0), "{read_only:?}");
}

/// 計画が未承認なら、ブリーフはドメインの理由を名指して拒否される。
///
/// 承認の判断そのものへ到達していることを観測する — 権限の解決で止まる形とは別である。
#[test]
fn a_worker_brief_without_an_approved_plan_names_the_domain_reason() {
    let workspace = workspace_at_code_generation();
    // 権限は発行済みの指示から解決するので、まず `next` を通す。
    let issued = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .arg("next")
        .current_dir(workspace.path())
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", workspace.path().join("aidlc/.capture-home"))
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    assert!(issued.status.success(), "{issued:?}");
    let output = worker_brief(&workspace);
    assert_ne!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains(
            "Cannot assemble a worker brief for the stage-level target: code-generation-plan.md is missing or empty"
        ),
        "承認の判断まで到達していない — {stderr}"
    );
    assert!(
        !stderr.contains("AIDLC-STAGE") && !stderr.contains("## Approved plan"),
        "承認が無いのにブリーフ本文が漏れた — {stderr}"
    );
}

#[test]
fn stop_observation_preserves_an_approved_code_generation_receipt() {
    let workspace = workspace_with_approved_plan();
    let approvals = workspace.path().join("aidlc/.aidlc-sessions/plan-approval");
    let files = || -> std::collections::BTreeMap<String, Vec<u8>> {
        fs::read_dir(&approvals)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (
                    entry.file_name().to_string_lossy().to_string(),
                    fs::read(entry.path()).unwrap(),
                )
            })
            .collect()
    };
    let before = files();
    assert!(!before.is_empty());
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let marker = record.join(".aidlc-active-directive.json");
    let marker_before = fs::read(&marker).unwrap();
    let source = rusqlite::Connection::open(intents.join(".aidlc-store.sqlite")).unwrap();
    let count = || {
        source
            .query_row::<i64, _, _>("SELECT count(*) FROM journal", [], |row| row.get(0))
            .unwrap()
    };
    let source_before = count();
    let stopped = workspace.stop(r#"{"session_id":"stage1-session"}"#, &[]);
    assert_eq!(stopped.status.code(), Some(0));
    assert!(stopped.stderr.is_empty(), "{stopped:?}");
    assert_eq!(files(), before, "Stopのnext照会で計画承認を失効させない");
    assert_eq!(fs::read(marker).unwrap(), marker_before);
    assert_eq!(count(), source_before, "指示発行や無効化をSourceへ書かない");
}
#[test]
fn begin_publishes_generation_and_rechecks_documents_on_repeated_calls() {
    let workspace = workspace_with_approved_plan();
    let begin = || {
        Command::new(workspace.path().join("bin/aidlc-testing-posture"))
            .args(["begin", "--stage-level"])
            .current_dir(workspace.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", workspace.path().join("aidlc/.capture-home"))
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap()
    };
    let runtime_dir = workspace.path().join("aidlc/.aidlc-sessions/plan-approval");
    let receipt_file = fs::read_dir(&runtime_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("receipt-")
        })
        .unwrap();
    let receipt_before = fs::read(&receipt_file).unwrap();
    let before = workspace.state();
    let first = begin();
    assert!(first.status.success(), "{first:?}");
    assert_eq!(
        first.stdout,
        b"{\"status\":\"generation\",\"target\":{\"unit\":null}}\n"
    );
    assert_eq!(workspace.state(), before);
    if let Ok(destination) = std::env::var("AIDLC_PLAN_BEGIN_CAPTURE") {
        use base64::Engine as _;
        let source = PathBuf::from(std::env::var("AIDLC_PLAN_DECISION_SOURCE").unwrap());
        let bun = PathBuf::from(std::env::var("AIDLC_PLAN_DECISION_BUN").unwrap());
        let native_receipt = fs::read(&receipt_file).unwrap();
        let input_receipt: serde_json::Value = serde_json::from_slice(&receipt_before).unwrap();
        assert_eq!(
            input_receipt
                .get("status")
                .and_then(serde_json::Value::as_str),
            Some("approved")
        );
        fs::write(&receipt_file, &receipt_before).unwrap();
        let upstream = Command::new(&bun)
            .arg(source.join(".claude/tools/aidlc-testing-posture.ts"))
            .args(["begin", "--stage-level"])
            .current_dir(workspace.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", workspace.path().join("aidlc/.capture-home"))
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", bun.parent().unwrap().display()),
            )
            .env("CLAUDE_PROJECT_DIR", workspace.path())
            .env("AIDLC_PROJECT_DIR", workspace.path())
            .output()
            .unwrap();
        let source_receipt = fs::read(&receipt_file).unwrap();
        let published: serde_json::Value = serde_json::from_slice(&source_receipt).unwrap();
        assert_eq!(
            published.get("status").and_then(serde_json::Value::as_str),
            Some("generation")
        );
        assert_ne!(
            source_receipt, receipt_before,
            "本家beginが入力approvedを更新する"
        );
        let intents = workspace.path().join("aidlc/spaces/default/intents");
        let record = intents.join(
            fs::read_to_string(intents.join("active-intent"))
                .unwrap()
                .trim(),
        );
        let documents = std::collections::BTreeMap::from_iter(
            [
                "code-generation-plan.md",
                "unit-test-instructions.md",
                "code-generation-questions.md",
            ]
            .map(|name| {
                (
                    name,
                    base64::engine::general_purpose::STANDARD.encode(
                        fs::read(record.join("construction/code-generation").join(name)).unwrap(),
                    ),
                )
            }),
        );
        let captured = capture_json::object([
            (
                "input",
                capture_json::object([
                    ("project_dir", capture_json::value(&workspace.path())),
                    ("argv", capture_json::value(&["begin", "--stage-level"])),
                    (
                        "state_base64",
                        capture_json::value(
                            &base64::engine::general_purpose::STANDARD.encode(&before),
                        ),
                    ),
                    ("documents_base64", capture_json::value(&documents)),
                    (
                        "receipt_before_base64",
                        capture_json::value(
                            &base64::engine::general_purpose::STANDARD.encode(&receipt_before),
                        ),
                    ),
                ]),
            ),
            (
                "native",
                capture_json::file_output(&first, "receipt_base64", &native_receipt),
            ),
            (
                "upstream",
                capture_json::file_output(&upstream, "receipt_base64", &source_receipt),
            ),
        ]);
        fs::write(destination, capture_json::pretty(&captured)).unwrap();
        assert!(upstream.status.success(), "{upstream:?}");
        assert_eq!(first.stdout, upstream.stdout);
        assert_eq!(first.stderr, upstream.stderr);
        assert_eq!(native_receipt, source_receipt, "開始許可の全受領バイト");
    }

    fs::write(
        workspace.path().join("implemented.rs"),
        "fn implemented() {}\n",
    )
    .unwrap();
    let repeated = begin();
    assert!(repeated.status.success(), "{repeated:?}");
    assert_eq!(repeated.stdout, first.stdout);
    let intents = workspace.path().join("aidlc/spaces/default/intents");
    let record = intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    );
    let questions = record.join("construction/code-generation/code-generation-questions.md");
    fs::write(&questions, fs::read_to_string(&questions).unwrap() + "\n").unwrap();
    let changed = begin();
    assert!(!changed.status.success(), "{changed:?}");
    assert!(
        String::from_utf8_lossy(&changed.stderr)
            .contains("no current protected Plan Approval receipt matches")
    );
}

#[test]
fn an_unfinished_generation_publication_is_recovered_before_other_approval_operations() {
    for source_changes in [false, true] {
        let workspace = workspace_with_approved_plan();
        let root = rusqlite::Connection::open(workspace.path().join("aidlc/.aidlc-runtime.sqlite"))
            .unwrap();
        root.execute_batch("CREATE TRIGGER fail_generation_certification BEFORE INSERT ON journal WHEN json_extract(CAST(NEW.payload AS TEXT),'$.payload.type')='GenerationCertified' BEGIN SELECT RAISE(ABORT,'injected generation certification failure'); END;").unwrap();
        let output = Command::new(workspace.path().join("bin/aidlc-testing-posture"))
            .args(["begin", "--stage-level"])
            .current_dir(workspace.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", workspace.path().join("aidlc/.capture-home"))
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap();
        assert!(
            !output.status.success(),
            "確定保存の失敗を成功として返さない"
        );
        assert!(output.stdout.is_empty());
        let (id, status): (String, String) = root
            .query_row(
                "SELECT operation_id,status FROM read_plan_generation",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, "pending");
        let directory = workspace.path().join("aidlc/.aidlc-sessions/plan-approval");
        let receipts = || {
            fs::read_dir(&directory)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| {
                    path.file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .starts_with("receipt-")
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(receipts().len(), 1);
        root.execute_batch("DROP TRIGGER fail_generation_certification")
            .unwrap();
        if source_changes {
            fs::write(
                workspace.path().join("changed-during-start.rs"),
                "fn changed() {}\n",
            )
            .unwrap();
        }
        assert!(
            workspace
                .human_prompt(r#"{"session_id":"other","prompt":"continue"}"#, false)
                .status
                .success()
        );
        let daos = core_query_interface_adapter::ReadModelDaos::open(
            &workspace.path().join("aidlc/.aidlc-runtime.sqlite"),
        )
        .unwrap();
        let query =
            core_query_use_case::orchestration::PlanGenerationUseCase::new(daos.plan_generation());
        let result = query.execute(&id).unwrap().unwrap();
        assert_eq!(
            result.status(),
            if source_changes {
                "revoked"
            } else {
                "generation"
            }
        );
        assert_eq!(
            result.error(),
            if source_changes {
                Some("workspace source changed while Code Generation authority was starting")
            } else {
                None
            }
        );
        assert_eq!(receipts().len(), usize::from(!source_changes));
        let terminal_events = || {
            root.query_row("SELECT count(*) FROM journal WHERE json_extract(CAST(payload AS TEXT),'$.payload.type') IN ('GenerationCertified','GenerationRevoked')",[],|row|row.get::<_,i64>(0)).unwrap()
        };
        assert_eq!(terminal_events(), 1);
        assert!(
            workspace
                .human_prompt(r#"{"session_id":"other","prompt":"again"}"#, false)
                .status
                .success()
        );
        assert_eq!(terminal_events(), 1, "回復済みの同じ開始を二度確定しない");
    }
}

#[test]
fn numeric_human_envelopes_preserve_only_string_number_choices() {
    let (workspace, questions) = workspace_with_plan_questions();
    let asked = workspace.log(&[
        "decision",
        "--stage",
        "code-generation",
        "--checkpoint",
        "plan-approval",
        "--questions-file",
        &questions,
        "--session",
        "stage1-session",
        "--stage-level",
        "--decision",
        "Approve this exact Code Generation plan?",
        "--options",
        "Approve Plan,Request Changes",
    ]);
    assert!(asked.status.success(), "{asked:?}");
    let number_input = r#"{"session_id":"stage1-session","prompt":1}"#;
    let string_input = r#"{"session_id":"stage1-session","prompt":"1"}"#;
    let response = workspace
        .path()
        .join("aidlc/.aidlc-sessions/plan-approval/response-stage1-session.json");
    assert!(!response.exists());
    let number = workspace.human_prompt(number_input, false);
    assert!(number.status.success());
    assert!(!response.exists(), "JSON数値は承認候補へ昇格させない");
    let string = workspace.human_prompt(string_input, false);
    assert!(string.status.success());
    let native_response = fs::read(&response).unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&native_response).unwrap();
    assert_eq!(
        parsed.get("choice").and_then(serde_json::Value::as_str),
        Some("Approve Plan")
    );
    if let Ok(destination) = std::env::var("AIDLC_NUMERIC_HUMAN_CAPTURE") {
        use std::io::Write as _;
        let source = PathBuf::from(std::env::var("AIDLC_PLAN_DECISION_SOURCE").unwrap());
        let bun = PathBuf::from(std::env::var("AIDLC_PLAN_DECISION_BUN").unwrap());
        let intents = workspace.path().join("aidlc/spaces/default/intents");
        let record = intents.join(
            fs::read_to_string(intents.join("active-intent"))
                .unwrap()
                .trim(),
        );
        let human_count = || {
            fs::read_dir(record.join("audit"))
                .unwrap()
                .map(|entry| {
                    fs::read_to_string(entry.unwrap().path())
                        .unwrap()
                        .matches("**Event**: HUMAN_TURN")
                        .count()
                })
                .sum::<usize>()
        };
        fs::remove_file(&response).unwrap();
        let mut observations = Vec::new();
        for (id, input, native, bytes) in [
            ("json-number", number_input, &number, None),
            (
                "string-number",
                string_input,
                &string,
                Some(native_response.as_slice()),
            ),
        ] {
            assert!(!response.exists(), "本家実行前にresponseがない");
            let before = human_count();
            let mut process = Command::new(&bun)
                .arg(source.join(".claude/hooks/aidlc-record-human-turn.ts"))
                .current_dir(workspace.path())
                .env_clear()
                .envs(coverage_profile_env())
                .env("HOME", workspace.path().join("aidlc/.capture-home"))
                .env(
                    "PATH",
                    format!("{}:/usr/bin:/bin", bun.parent().unwrap().display()),
                )
                .env("CLAUDE_PROJECT_DIR", workspace.path())
                .env("AIDLC_PROJECT_DIR", workspace.path())
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            process
                .stdin
                .take()
                .unwrap()
                .write_all(input.as_bytes())
                .unwrap();
            let upstream = process.wait_with_output().unwrap();
            assert!(upstream.status.success(), "{upstream:?}");
            assert_eq!(
                human_count(),
                before + 1,
                "本家フックの新しい実行事実を確認する"
            );
            assert!(
                !response.exists(),
                "未修正の本家では両数値入力とも保護responseは作られない"
            );
            observations.push(capture_json::object([
                ("id", capture_json::value(&id)),
                ("stdin", capture_json::value(&input)),
                (
                    "native",
                    capture_json::optional_file_output(native, "response_base64", bytes),
                ),
                (
                    "upstream",
                    capture_json::optional_file_output(&upstream, "response_base64", None),
                ),
                ("upstream_new_human_turn", capture_json::value(&true)),
            ]));
        }
        fs::write(
            destination,
            capture_json::pretty(&core_infrastructure::canon_json::JsonValue::Array(
                observations,
            )),
        )
        .unwrap();
    }
}

// ---------------------------------------------------------------------------
// u2_cov2_app: 計画承認の周辺 — rationale の同伴、承認前の begin、壊れた質問文書
// ---------------------------------------------------------------------------

#[test]
fn a_plan_decision_carries_its_rationale_and_begin_is_refused_before_approval() {
    let (workspace, questions) = workspace_with_plan_questions();
    let begin = || {
        Command::new(workspace.path().join("bin/aidlc-testing-posture"))
            .args(["begin", "--stage-level"])
            .current_dir(workspace.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", workspace.path().join("aidlc/.capture-home"))
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap()
    };
    // 提示も承認も無いうちは生成の開始を認証しない。
    let premature = begin();
    assert_eq!(premature.status.code(), Some(1), "{premature:?}");
    let refusal: serde_json::Value = serde_json::from_slice(&premature.stderr).unwrap();
    assert_eq!(
        refusal.get("error").and_then(serde_json::Value::as_str),
        Some("Plan Approval is not explicitly answered Approve Plan")
    );
    let asked = workspace.log(&[
        "decision",
        "--stage",
        "code-generation",
        "--checkpoint",
        "plan-approval",
        "--questions-file",
        &questions,
        "--session",
        "stage1-session",
        "--stage-level",
        "--decision",
        "Approve this exact Code Generation plan?",
        "--options",
        "Approve Plan,Request Changes",
        "--rationale",
        "The plan matches the approved units.",
    ]);
    assert!(asked.status.success(), "{asked:?}");
    let audit: String = fs::read_dir(workspace.record_dir().join("audit"))
        .unwrap()
        .map(|entry| fs::read_to_string(entry.unwrap().path()).unwrap())
        .collect();
    assert!(
        audit.contains("The plan matches the approved units."),
        "rationale は監査へ同伴する: {audit}"
    );
    // 提示だけでは生成を始められない（人間の応答と受領が要る）。
    let offered_only = begin();
    assert_eq!(offered_only.status.code(), Some(1), "{offered_only:?}");
    // 文字を持たないセッション名は提示にも受領にも使えない。
    let blank_session = |verb: &str, tail: &[&str]| {
        let mut args = vec![
            verb,
            "--stage",
            "code-generation",
            "--checkpoint",
            "plan-approval",
            "--questions-file",
            &questions,
            "--session",
            "!!!",
            "--stage-level",
        ];
        args.extend_from_slice(tail);
        let refused = workspace.log(&args);
        assert_eq!(refused.status.code(), Some(1), "{refused:?}");
        let error: serde_json::Value = serde_json::from_slice(&refused.stderr).unwrap();
        assert_eq!(
            error.get("error").and_then(serde_json::Value::as_str),
            Some("Plan Approval challenge requires a nonblank session"),
            "{verb}"
        );
    };
    blank_session(
        "decision",
        &[
            "--decision",
            "Approve?",
            "--options",
            "Approve Plan,Request Changes",
        ],
    );
    blank_session("answer", &["--details", "Approve Plan"]);
}

#[test]
fn a_plan_answer_needs_a_readable_questions_file_at_the_supplied_path() {
    let (workspace, questions) = workspace_with_plan_questions();
    let asked = workspace.log(&[
        "decision",
        "--stage",
        "code-generation",
        "--checkpoint",
        "plan-approval",
        "--questions-file",
        &questions,
        "--session",
        "stage1-session",
        "--stage-level",
        "--decision",
        "Approve this exact Code Generation plan?",
        "--options",
        "Approve Plan,Request Changes",
    ]);
    assert!(asked.status.success(), "{asked:?}");
    assert!(
        workspace
            .human_prompt(r#"{"session_id":"stage1-session","prompt":"1"}"#, false)
            .status
            .success()
    );
    let file = workspace.path().join(&questions);
    fs::remove_file(&file).unwrap();
    fs::create_dir(&file).unwrap();
    let answered = workspace.log(&[
        "answer",
        "--stage",
        "code-generation",
        "--checkpoint",
        "plan-approval",
        "--questions-file",
        &questions,
        "--session",
        "stage1-session",
        "--stage-level",
        "--details",
        "Approve Plan",
    ]);
    assert_eq!(answered.status.code(), Some(1), "{answered:?}");
    let refusal: serde_json::Value = serde_json::from_slice(&answered.stderr).unwrap();
    let error = refusal
        .get("error")
        .and_then(serde_json::Value::as_str)
        .unwrap();
    assert!(
        error.starts_with("steering read: IsADirectory at ")
            && error.ends_with("construction/code-generation/code-generation-questions.md"),
        "{error}"
    );
    assert!(
        !workspace
            .path()
            .join("aidlc/.aidlc-sessions/plan-approval")
            .join("receipt-stage1-session.json")
            .exists(),
        "読めない質問文書では受領証を出さない"
    );
}

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;
