//! 固定本家のClaudeフックを、保存済みstdinと全出力で照合する。
#![allow(clippy::unwrap_used)]
use std::{
    fs,
    io::Write as _,
    process::{Command, Stdio},
};

/// 人間裁定の1ケースだけ、Bun固有診断を保存した上でRustの固定診断を照合する。
#[test]
fn heartbeat_directory_failure_preserves_public_files_and_reports_eisdir() {
    use base64::Engine as _;
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/write-audit-health.json"
    ))
    .unwrap();
    let case = corpus
        .get("observations")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|case| {
            case.get("id").and_then(serde_json::Value::as_str) == Some("heartbeat-directory/1")
        })
        .unwrap();
    let input = case.get("input").unwrap();
    let environment = input.get("environment").unwrap();
    let captured_root = environment
        .get("AIDLC_PROJECT_DIR")
        .unwrap()
        .as_str()
        .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().canonicalize().unwrap();
    let initial = case.get("initial_files").unwrap().as_object().unwrap();
    for (relative, encoded) in initial {
        let path = project.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            path,
            base64::engine::general_purpose::STANDARD
                .decode(encoded.as_str().unwrap())
                .unwrap(),
        )
        .unwrap();
    }
    let relative = case
        .get("fixture_directories")
        .unwrap()
        .as_array()
        .unwrap()
        .first()
        .unwrap()
        .as_str()
        .unwrap();
    let heartbeat = project.join(relative);
    fs::create_dir_all(&heartbeat).unwrap();
    let public_before = public_aidlc_files(&project);
    let output = case.get("output").unwrap();
    let upstream_stderr = output.get("stderr").unwrap().as_str().unwrap();
    let captured_error =
        format!("EISDIR: illegal operation on a directory, open '{captured_root}/{relative}'");
    assert!(upstream_stderr.lines().any(|line| line == captured_error));
    assert!(
        upstream_stderr.contains("syscall: \"open\"")
            && upstream_stderr.contains("code: \"EISDIR\"")
    );
    assert_eq!(output.get("exit_code").unwrap().as_i64(), Some(1));
    assert_eq!(output.get("stdout").unwrap().as_str(), Some(""));
    assert!(
        case.get("changed_files")
            .unwrap()
            .as_object()
            .unwrap()
            .is_empty()
    );
    let stdin = input
        .get("stdin")
        .unwrap()
        .as_str()
        .unwrap()
        .replace(captured_root, &project.to_string_lossy());
    let mut child = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .args(["hook", "write-audit-log"])
        .current_dir(&project)
        .env_clear()
        .env("HOME", &project)
        .env("PATH", "/usr/bin:/bin")
        .envs(coverage_profile_env())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let actual = child.wait_with_output().unwrap();
    assert_eq!(actual.status.code(), Some(1));
    assert!(actual.stdout.is_empty());
    assert_eq!(
        String::from_utf8(actual.stderr).unwrap(),
        format!(
            "EISDIR: illegal operation on a directory, open '{}'\n",
            heartbeat.display()
        )
    );
    for (relative, encoded) in initial {
        assert_eq!(
            fs::read(project.join(relative)).unwrap(),
            base64::engine::general_purpose::STANDARD
                .decode(encoded.as_str().unwrap())
                .unwrap(),
            "{relative}"
        );
    }
    assert!(heartbeat.is_dir());
    assert_eq!(fs::read_dir(&heartbeat).unwrap().count(), 0);
    assert_eq!(
        fs::read_dir(heartbeat.parent().unwrap()).unwrap().count(),
        1,
        "dropやheartbeatファイルを生成しない"
    );
    assert_eq!(
        public_aidlc_files(&project),
        public_before,
        "別の監査shardを含む公開ファイル集合も変わらない"
    );
}

/// SQLiteとその共有初期化markerだけはRust内部。その他の公開ファイルは全件比較する。
fn public_aidlc_files(
    project: &std::path::Path,
) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    let mut result = std::collections::BTreeMap::new();
    let mut pending = vec![project.join("aidlc")];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                pending.push(path);
            } else {
                let relative = path.strip_prefix(project).unwrap();
                if ![
                    "aidlc/.aidlc-runtime.sqlite",
                    "aidlc/.aidlc-runtime.sqlite-wal",
                    "aidlc/.aidlc-runtime.sqlite-shm",
                    "aidlc/.aidlc-runtime.state.json",
                ]
                .iter()
                .any(|internal| relative == std::path::Path::new(internal))
                {
                    result.insert(relative.to_path_buf(), fs::read(&path).unwrap());
                }
            }
        }
    }
    result
}

#[test]
fn state_transition_guard_matches_the_four_fixed_upstream_boundaries() {
    let corpus = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/golden/upstream-a277af21/hooks/state-transition-guard");
    for name in [
        "deny-direct-state-transition",
        "deny-delegated-lifecycle",
        "ignore-non-bash-tool",
        "allow-read-only-query",
    ] {
        let case = corpus.join(name);
        let temp = tempfile::tempdir().unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .args(["hook", "state-transition-guard"])
            .current_dir(temp.path())
            .env_clear()
            .env("HOME", temp.path())
            .env("PATH", "/usr/bin:/bin")
            .envs(coverage_profile_env())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&fs::read(case.join("stdin.json")).unwrap())
            .unwrap();
        let actual = child.wait_with_output().unwrap();
        let exit = fs::read_to_string(case.join("exit"))
            .unwrap()
            .trim()
            .parse::<i32>()
            .unwrap();
        assert_eq!(actual.status.code(), Some(exit), "{name}: {actual:?}");
        assert_eq!(
            actual.stdout,
            fs::read(case.join("stdout")).unwrap(),
            "{name}"
        );
        assert_eq!(
            actual.stderr,
            fs::read(case.join("stderr")).unwrap(),
            "{name}"
        );
        assert_eq!(
            fs::read_dir(temp.path()).unwrap().count(),
            0,
            "純粋な保護判定で初期化を起こさない"
        );
    }
}

#[test]
fn continue_hook_without_workflow_allows_stop_after_recording_health() {
    let corpus = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../../tests/golden/upstream-a277af21/hooks/stop-forwarding-loop/no-workflow-ignored",
    );
    for stdin in [
        fs::read(corpus.join("stdin.json")).unwrap(),
        b"{".to_vec(),
        b"null".to_vec(),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .args(["hook", "continue-workflow"])
            .current_dir(temp.path())
            .env_clear()
            .env("HOME", temp.path())
            .env("PATH", "/usr/bin:/bin")
            .envs(coverage_profile_env())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(&stdin).unwrap();
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert_eq!(output.stdout, fs::read(corpus.join("stdout")).unwrap());
        assert_eq!(output.stderr, fs::read(corpus.join("stderr")).unwrap());
        let heartbeat = temp
            .path()
            .join("aidlc/spaces/default/intents/.aidlc-hooks-health/continue-workflow.last");
        assert!(
            chrono::DateTime::parse_from_rfc3339(&fs::read_to_string(heartbeat).unwrap()).is_ok()
        );
        assert!(
            !temp
                .path()
                .join("aidlc/spaces/default/intents/active-intent")
                .exists()
        );
        assert!(
            !temp
                .path()
                .join("aidlc/spaces/default/intents/.aidlc-store.sqlite")
                .exists()
        );
    }
}

#[test]
fn main_state_guard_matches_fixed_shell_boundaries() {
    assert_shell_mode("main");
}

#[test]
fn delegated_state_guard_matches_fixed_shell_boundaries() {
    assert_shell_mode("delegated");
}

fn assert_shell_mode(mode: &str) {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/state-guard-shell.json"
    ))
    .unwrap();
    let cases = corpus.get("observations").unwrap().as_array().unwrap();
    let mut count = 0;
    for case in cases
        .iter()
        .filter(|case| case.get("mode").and_then(serde_json::Value::as_str) == Some(mode))
    {
        count += 1;
        let input = case.get("stdin").unwrap().as_str().unwrap();
        let actual = harness_claude::StateTransitionGuard::evaluate(input).unwrap();
        let stderr = actual
            .denial()
            .map_or_else(String::new, |reason| format!("{reason}\n"));
        assert_eq!(
            stderr,
            case.get("stderr").unwrap().as_str().unwrap(),
            "{}",
            case.get("id").unwrap()
        );
        assert_eq!(
            if actual.denial().is_some() { 2 } else { 0 },
            case.get("exit").unwrap().as_i64().unwrap()
        );
    }
    assert!(count > 0);
}

#[test]
fn invalid_state_guard_envelopes_match_fixed_source() {
    assert_shell_mode("envelope");
}

#[test]
fn state_guard_ignores_invalid_utf8_stdin_without_initialization() {
    let temp = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .args(["hook", "state-transition-guard"])
        .current_dir(temp.path())
        .env_clear()
        .env("HOME", temp.path())
        .env("PATH", "/usr/bin:/bin")
        .envs(coverage_profile_env())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&[0xff]).unwrap();
    let result = child.wait_with_output().unwrap();
    assert_eq!(result.status.code(), Some(0));
    assert!(result.stdout.is_empty() && result.stderr.is_empty());
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
}

#[test]
fn write_audit_hook_records_a_heartbeat_in_the_shared_runtime_store() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("aidlc")).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .args(["hook", "write-audit-log"])
        .current_dir(temp.path())
        .env_clear()
        .env("HOME", temp.path())
        .env("PATH", "/usr/bin:/bin")
        .envs(coverage_profile_env())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"{").unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
    let db = rusqlite::Connection::open(temp.path().join("aidlc/.aidlc-runtime.sqlite")).unwrap();
    assert_eq!(
        db.query_row::<i64, _, _>(
            "SELECT count(*) FROM journal WHERE manifest='hook-health-event/1'",
            [],
            |row| row.get(0)
        )
        .unwrap(),
        1
    );
    assert!(
        temp.path()
            .join("aidlc/spaces/default/intents/.aidlc-hooks-health/write-audit-log.last")
            .exists()
    );
    let mut second = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .args(["hook", "write-audit-log"])
        .current_dir(temp.path())
        .env_clear()
        .env("HOME", temp.path())
        .env("PATH", "/usr/bin:/bin")
        .envs(coverage_profile_env())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    second.stdin.take().unwrap().write_all(b"null").unwrap();
    let second_output = second.wait_with_output().unwrap();
    assert_eq!(second_output.status.code(), Some(0));
    assert!(second_output.stdout.is_empty() && second_output.stderr.is_empty());
    assert_eq!(
        rusqlite::Connection::open(temp.path().join("aidlc/.aidlc-runtime.sqlite"))
            .unwrap()
            .query_row::<i64, _, _>(
                "SELECT count(*) FROM journal WHERE manifest='hook-health-event/1'",
                [],
                |row| row.get(0),
            )
            .unwrap(),
        2
    );
}

/// `audit/` の `.md` のうち `marker` を含む唯一のシャードを返す。
fn projected_shard(audit: &std::path::Path, marker: &str) -> std::path::PathBuf {
    let mut shards: Vec<std::path::PathBuf> = fs::read_dir(audit)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .filter(|path| fs::read_to_string(path).unwrap().contains(marker))
        .collect();
    assert_eq!(
        shards.len(),
        1,
        "{marker} を含むシャードは 1 つ: {shards:?}"
    );
    shards.remove(0)
}

#[test]
fn write_audit_hook_projects_created_updated_and_drop_via_events() {
    let temp = tempfile::tempdir().unwrap();
    let record = temp
        .path()
        .join("aidlc/spaces/default/intents/260908-hook-health");
    fs::create_dir_all(record.join("audit")).unwrap();
    fs::write(temp.path().join("aidlc/active-space"), "default").unwrap();
    fs::write(
        temp.path()
            .join("aidlc/spaces/default/intents/active-intent"),
        "260908-hook-health",
    )
    .unwrap();
    // カーソルが名指すディレクトリは `aidlc-state.md` を持って初めて記録である
    // (`Layout::shared`、upstream `activeIntent` と同じ判定、裁定 F-H1 = B)。状態ファイルが
    // 在るので監査はクローン別シャード (`<host>-<clone>.md`) へ投影される。フックの前提
    // 検査 (`audit/` 内に `.md` が 1 つ以上) のために空の `shard.md` を置くが、投影先は
    // 実測で `audit/*.md` から探す。
    fs::write(record.join("aidlc-state.md"), "# AI-DLC State\n").unwrap();
    fs::write(record.join("audit/shard.md"), "").unwrap();
    let artifact = record.join("artifact.md");
    fs::write(&artifact, "# artifact\n").unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .args(["hook", "write-audit-log"])
        .current_dir(temp.path())
        .env_clear()
        .env("HOME", temp.path())
        .env("PATH", "/usr/bin:/bin")
        .envs(coverage_profile_env())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            format!(
                r#"{{"tool_name":"Write","tool_input":{{"file_path":"{}"}}}}"#,
                artifact.display()
            )
            .as_bytes(),
        )
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let shard = projected_shard(&record.join("audit"), "**Event**: ARTIFACT_CREATED");
    let query_db = record.parent().unwrap().to_path_buf();
    let target = core_command_domain::workspace::HookHealthTarget::new(
        core_command_domain::workspace::SpaceName::parse("default").unwrap(),
        Some(core_command_domain::workspace::IntentDirName::parse("260908-hook-health").unwrap()),
    );
    let artifact_id = core_command_domain::workspace::ArtifactAuditId::for_target(&target);
    let daos = core_query_interface_adapter::ReadModelDaos::open(
        query_db.join(".aidlc-store.sqlite").as_path(),
    )
    .unwrap();
    let view = core_query_use_case::orchestration::ArtifactAuditUseCase::new(daos.artifact_audit())
        .execute(artifact_id.as_str())
        .unwrap()
        .unwrap();
    assert!(view.created());
    assert_eq!(view.tool(), "Write");
    fs::write(&artifact, "# artifact updated\n").unwrap();
    let mut edit = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .args(["hook", "write-audit-log"])
        .current_dir(temp.path())
        .env_clear()
        .env("HOME", temp.path())
        .env("PATH", "/usr/bin:/bin")
        .envs(coverage_profile_env())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    edit.stdin
        .take()
        .unwrap()
        .write_all(
            format!(
                r#"{{"tool_name":"Edit","tool_input":{{"file_path":"{}"}}}}"#,
                artifact.display()
            )
            .as_bytes(),
        )
        .unwrap();
    let edit_output = edit.wait_with_output().unwrap();
    assert_eq!(edit_output.status.code(), Some(0));
    let audit = fs::read_to_string(&shard).unwrap();
    assert!(audit.contains("**Event**: ARTIFACT_UPDATED"));
    assert_eq!(
        audit.matches("**Event**: ARTIFACT_CREATED").count(),
        1,
        "過去の作成監査を再度公開しない"
    );
    assert_eq!(audit.matches("**Event**: ARTIFACT_UPDATED").count(), 1);
    let updated =
        core_query_use_case::orchestration::ArtifactAuditUseCase::new(daos.artifact_audit())
            .execute(artifact_id.as_str())
            .unwrap()
            .unwrap();
    assert_eq!(updated.tool(), "Edit", "2回目も構造化投影を確定する");
    assert!(!updated.created());
    assert!(
        !record
            .join(".aidlc-hooks-health/write-audit-log.drops")
            .exists(),
        "正常な2回の保存で監査失敗を記録しない"
    );
    fs::remove_file(&shard).unwrap();
    fs::create_dir(&shard).unwrap();
    let mut failed = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .args(["hook", "write-audit-log"])
        .current_dir(temp.path())
        .env_clear()
        .env("HOME", temp.path())
        .env("PATH", "/usr/bin:/bin")
        .envs(coverage_profile_env())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    failed
        .stdin
        .take()
        .unwrap()
        .write_all(
            format!(
                r#"{{"tool_name":"Write","tool_input":{{"file_path":"{}"}}}}"#,
                artifact.display()
            )
            .as_bytes(),
        )
        .unwrap();
    let failed_output = failed.wait_with_output().unwrap();
    assert_eq!(failed_output.status.code(), Some(0));
    assert!(
        record
            .join(".aidlc-hooks-health/write-audit-log.drops")
            .exists()
    );
}

#[test]
fn native_intent_create_then_artifact_hook_uses_the_event_pipeline() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(project.join(".claude/tools/data")).unwrap();
    fs::create_dir_all(project.join("bin")).unwrap();
    fs::create_dir_all(project.join(".claude/scopes")).unwrap();
    fs::create_dir_all(project.join("aidlc/spaces/default/intents")).unwrap();
    let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    for name in ["stage-graph.json", "scope-grid.json", "harness.json"] {
        fs::copy(
            repo.join("tests/golden/upstream-a277af21/data").join(name),
            project.join(".claude/tools/data").join(name),
        )
        .unwrap();
    }
    fs::copy(
        repo.join(".claude/scopes/aidlc-bugfix.md"),
        project.join(".claude/scopes/aidlc-bugfix.md"),
    )
    .unwrap();
    tool_link::link_tool(&project.join("aidlc-utility")).unwrap();
    tool_link::link_tool(&project.join("bin/aidlc")).unwrap();
    let created = Command::new(project.join("aidlc-utility"))
        .args([
            "intent-create",
            "--scope",
            "bugfix",
            "--label",
            "native",
            "--arguments",
            "Fix one native artifact",
        ])
        .current_dir(&project)
        .env_clear()
        .env("HOME", &project)
        .env("PATH", "/usr/bin:/bin")
        .envs(coverage_profile_env())
        .output()
        .unwrap();
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
    let intents = project.join("aidlc/spaces/default/intents");
    let record_name = fs::read_to_string(intents.join("active-intent")).unwrap();
    let record = intents.join(record_name.trim());
    let artifact = record.join("construction/native-artifact.md");
    fs::create_dir_all(artifact.parent().unwrap()).unwrap();
    fs::write(&artifact, "# native\n").unwrap();
    for tool in ["Write", "Edit"] {
        let input = format!(
            r#"{{"tool_name":"{tool}","tool_input":{{"file_path":"{}"}}}}"#,
            artifact.display()
        );
        let mut hook = Command::new(project.join("bin/aidlc"))
            .args(["hook", "write-audit-log"])
            .current_dir(&project)
            .env_clear()
            .env("HOME", &project)
            .env("PATH", "/usr/bin:/bin")
            .envs(coverage_profile_env())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        hook.stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let output = hook.wait_with_output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty() && output.stderr.is_empty());
    }
    let audit = fs::read_to_string(
        record.join("audit").join(
            fs::read_dir(record.join("audit"))
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .file_name(),
        ),
    )
    .unwrap();
    assert_eq!(audit.matches("**Event**: ARTIFACT_CREATED").count(), 1);
    assert_eq!(audit.matches("**Event**: ARTIFACT_UPDATED").count(), 1);
    assert!(
        !record
            .join(".aidlc-hooks-health/write-audit-log.drops")
            .exists()
    );
    let target = core_command_domain::workspace::HookHealthTarget::new(
        core_command_domain::workspace::SpaceName::default(),
        Some(core_command_domain::workspace::IntentDirName::parse(record_name.trim()).unwrap()),
    );
    let daos = core_query_interface_adapter::ReadModelDaos::open(
        intents.join(".aidlc-store.sqlite").as_path(),
    )
    .unwrap();
    let view = core_query_use_case::orchestration::ArtifactAuditUseCase::new(daos.artifact_audit())
        .execute(core_command_domain::workspace::ArtifactAuditId::for_target(&target).as_str())
        .unwrap()
        .unwrap();
    assert_eq!(view.tool(), "Edit");
    assert!(!view.created());
    assert!(
        project
            .join("aidlc/spaces/default/intents/.aidlc-hooks-health/write-audit-log.last")
            .exists()
            || record
                .join(".aidlc-hooks-health/write-audit-log.last")
                .exists()
    );
}

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;

#[path = "../../../../tests/support/tool_link.rs"]
mod tool_link;
