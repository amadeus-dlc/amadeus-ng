//! 通常Claudeのセッション補助フックを固定本家の実観測へ合わせる。
#![allow(clippy::unwrap_used)]
use base64::Engine as _;
use std::{
    fs,
    io::Write as _,
    path::Path,
    process::{Command, Stdio},
};

fn case(id: &str) -> serde_json::Value {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/session-hooks.json"
    ))
    .unwrap();
    assert_eq!(
        corpus
            .get("source")
            .unwrap()
            .get("commit")
            .unwrap()
            .as_str(),
        Some("a277af218f0df7f325d3b8be7b6d90fce2c5bd40")
    );
    corpus
        .get("observations")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value.get("id").and_then(serde_json::Value::as_str) == Some(id))
        .unwrap()
        .clone()
}

fn restore(root: &Path, case: &serde_json::Value) {
    let captured_root = case
        .get("input")
        .unwrap()
        .get("environment")
        .unwrap()
        .get("AIDLC_PROJECT_DIR")
        .unwrap()
        .as_str()
        .unwrap();
    fs::create_dir_all(root.join(".claude")).unwrap();
    for (relative, encoded) in case.get("initial_files").unwrap().as_object().unwrap() {
        let Some(encoded) = encoded.as_str() else {
            continue;
        };
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap();
        let text = String::from_utf8(bytes)
            .unwrap()
            .replace(captured_root, &root.to_string_lossy());
        let path = root.join(local_path(relative, case));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
}

fn invoke(root: &Path, hook: &str, input: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .args(["hook", hook])
        .current_dir(root)
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", root)
        .env("PATH", "/usr/bin:/bin")
        .env("AIDLC_DISABLE_USAGE_TRACKING", "1")
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

#[test]
fn inactive_session_end_and_subagent_inputs_remain_without_workflow_updates() {
    for (id, hook) in [
        ("cold/end", "session-end"),
        ("cold/subagent", "log-subagent"),
        ("active/subagent-malformed", "log-subagent"),
    ] {
        let case = case(id);
        let root = tempfile::tempdir().unwrap();
        restore(root.path(), &case);
        let input = case
            .get("input")
            .unwrap()
            .get("stdin")
            .unwrap()
            .as_str()
            .unwrap();
        let expected = case.get("output").unwrap();
        let actual = invoke(root.path(), hook, input);
        assert_eq!(
            actual.status.code(),
            expected
                .get("exit_code")
                .and_then(serde_json::Value::as_i64)
                .map(|code| i32::try_from(code).unwrap()),
            "{id}: {actual:?}"
        );
        assert_eq!(
            actual.stdout,
            expected.get("stdout").unwrap().as_str().unwrap().as_bytes(),
            "{id}"
        );
        assert_eq!(
            actual.stderr,
            expected.get("stderr").unwrap().as_str().unwrap().as_bytes(),
            "{id}"
        );
        assert!(
            !root.path().join("aidlc/.aidlc-runtime.sqlite").exists(),
            "無視する入力で内部ストアを作らない: {id}"
        );
        assert!(
            case.get("changed_files")
                .unwrap()
                .as_object()
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn a_running_subagent_completion_appends_the_fixed_audit_without_changing_state() {
    assert_audit_case("active/subagent", "log-subagent");
}

fn assert_audit_case(id: &str, hook: &str) {
    let case = case(id);
    let root = tempfile::tempdir().unwrap();
    restore(root.path(), &case);
    let input = case
        .get("input")
        .unwrap()
        .get("stdin")
        .unwrap()
        .as_str()
        .unwrap();
    let expected = case.get("output").unwrap();
    let actual = invoke(root.path(), hook, input);
    assert_eq!(actual.status.code(), Some(0), "{actual:?}");
    assert_eq!(
        actual.stdout,
        expected.get("stdout").unwrap().as_str().unwrap().as_bytes()
    );
    assert_eq!(
        actual.stderr,
        expected.get("stderr").unwrap().as_str().unwrap().as_bytes()
    );
    let mut expected_audits = std::collections::BTreeMap::new();
    for collection in ["initial_files", "changed_files"] {
        for (path, encoded) in case.get(collection).unwrap().as_object().unwrap() {
            if path.contains("/audit/") && path.ends_with(".md") {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(encoded.as_str().unwrap())
                    .unwrap();
                expected_audits.insert(
                    local_path(path, &case),
                    normalize_clock(&String::from_utf8(bytes).unwrap()),
                );
            }
        }
    }
    for (path, expected) in &expected_audits {
        assert_eq!(
            normalize_clock(&fs::read_to_string(root.path().join(path)).unwrap()),
            *expected,
            "{id}: {path:?}"
        );
    }

    for (path, encoded) in case.get("initial_files").unwrap().as_object().unwrap() {
        if path.ends_with("/aidlc-state.md") {
            let expected = base64::engine::general_purpose::STANDARD
                .decode(encoded.as_str().unwrap())
                .unwrap();
            assert_eq!(fs::read(root.path().join(path)).unwrap(), expected);
        }
    }
    for (path, encoded) in case.get("changed_files").unwrap().as_object().unwrap() {
        if path.ends_with("/.aidlc-recovery.md") {
            let expected = String::from_utf8(
                base64::engine::general_purpose::STANDARD
                    .decode(encoded.as_str().unwrap())
                    .unwrap(),
            )
            .unwrap();
            let actual = fs::read_to_string(root.path().join(path)).unwrap();
            let normalize = |text: &str| {
                text.lines()
                    .map(|line| {
                        if let Some(clock) = line.strip_prefix("**Last validated**: ") {
                            assert!(chrono::DateTime::parse_from_rfc3339(clock).is_ok());
                            "**Last validated**: <TIME>"
                        } else {
                            line
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            assert_eq!(normalize(&actual), normalize(&expected), "{id}: breadcrumb");
        }
    }
}

#[test]
fn a_bound_session_end_appends_to_its_own_audit_without_completing_the_workflow() {
    assert_audit_case("active/bound-end", "session-end");
}

#[test]
fn session_end_distinguishes_an_unbound_session_from_an_anonymous_notification() {
    assert_audit_case("active/unbound-end", "session-end");
    assert_audit_case("active/anonymous-end", "session-end");
}

#[test]
fn subagent_message_truncation_and_optional_fields_match_the_fixed_source() {
    assert_audit_case("active/subagent-long-message", "log-subagent");
    assert_audit_case("active/subagent-missing-fields", "log-subagent");
}

#[test]
fn a_completed_workflow_ignores_subagent_completion_without_a_heartbeat() {
    let case = case("completed/subagent");
    let root = tempfile::tempdir().unwrap();
    restore(root.path(), &case);
    let output = invoke(
        root.path(),
        "log-subagent",
        case.get("input")
            .unwrap()
            .get("stdin")
            .unwrap()
            .as_str()
            .unwrap(),
    );
    assert!(output.status.success() && output.stdout.is_empty() && output.stderr.is_empty());
    assert!(
        case.get("changed_files")
            .unwrap()
            .as_object()
            .unwrap()
            .is_empty()
    );
    for path in case
        .get("initial_files")
        .unwrap()
        .as_object()
        .unwrap()
        .keys()
        .filter(|path| path.ends_with("aidlc-state.md"))
    {
        assert!(
            !root
                .path()
                .join(path)
                .parent()
                .unwrap()
                .join(".aidlc-hooks-health/log-subagent.last")
                .exists()
        );
    }
}

#[test]
fn a_cold_session_start_preserves_identity_without_creating_a_workflow() {
    let case = case("cold/startup");
    let root = tempfile::tempdir().unwrap();
    restore(root.path(), &case);
    let output = invoke(
        root.path(),
        "session-start",
        case.get("input")
            .unwrap()
            .get("stdin")
            .unwrap()
            .as_str()
            .unwrap(),
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        output.stdout,
        case.get("output")
            .unwrap()
            .get("stdout")
            .unwrap()
            .as_str()
            .unwrap()
            .as_bytes()
    );
    assert!(output.stderr.is_empty());
    assert_eq!(
        fs::read_to_string(root.path().join("aidlc/.aidlc-sessions/.current-session")).unwrap(),
        "session-a\n"
    );
    let binding: serde_json::Value = serde_json::from_slice(
        &fs::read(
            root.path()
                .join("aidlc/.aidlc-sessions/session-a.binding.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        binding.get("space").and_then(serde_json::Value::as_str),
        Some("default")
    );
    assert_eq!(binding.get("intent"), Some(&serde_json::Value::Null));
    assert!(
        chrono::DateTime::parse_from_rfc3339(binding.get("boundAt").unwrap().as_str().unwrap())
            .is_ok()
    );
    assert_eq!(
        fs::read_to_string(root.path().join("aidlc/active-space")).unwrap(),
        "default\n"
    );
    assert!(
        !root
            .path()
            .join("aidlc/spaces/default/intents/active-intent")
            .exists()
    );
    assert!(!root.path().join("aidlc/.aidlc-runtime.sqlite").exists());
}

fn create_intent(
    root: &Path,
    label: &str,
    scope: &str,
    session: Option<&str>,
) -> std::process::Output {
    let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let data = root.join(".claude/tools/data");
    fs::create_dir_all(&data).unwrap();
    for name in ["stage-graph.json", "scope-grid.json", "harness.json"] {
        fs::copy(
            repo.join("tests/golden/upstream-a277af21/data").join(name),
            data.join(name),
        )
        .unwrap();
    }
    fs::create_dir_all(root.join(".claude/scopes")).unwrap();
    fs::copy(
        repo.join(".claude/scopes/aidlc-bugfix.md"),
        root.join(".claude/scopes/aidlc-bugfix.md"),
    )
    .unwrap();
    let binaries = tempfile::tempdir().unwrap();
    let utility = binaries.path().join("aidlc-utility");
    tool_link::link_tool(&utility).unwrap();
    let mut command = Command::new(utility);
    command
        .args([
            "intent-create",
            "--scope",
            scope,
            "--label",
            label,
            "--arguments",
            "Fix a small defect",
        ])
        .current_dir(root)
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", root)
        .env("PATH", "/usr/bin:/bin")
        .env("AIDLC_TEST_SESSION_PLATFORM", "win32");
    if let Some(session) = session {
        command.env("AIDLC_SESSION_OVERRIDE", session);
    }
    command.output().unwrap()
}

#[test]
fn the_creating_session_rebinds_its_cold_navigation_to_the_new_intent() {
    let case = case("cold/startup");
    let root = tempfile::tempdir().unwrap();
    restore(root.path(), &case);
    let started = invoke(
        root.path(),
        "session-start",
        r#"{"source":"startup","session_id":"session-a"}"#,
    );
    assert!(started.status.success());
    let another = invoke(
        root.path(),
        "session-start",
        r#"{"source":"startup","session_id":"session-b"}"#,
    );
    assert!(another.status.success());
    let created = create_intent(root.path(), "first-session", "bugfix", Some("session-a"));
    assert!(created.status.success(), "{created:?}");
    let intents = root.path().join("aidlc/spaces/default/intents");
    let current = fs::read_to_string(intents.join("active-intent")).unwrap();
    let binding: serde_json::Value = serde_json::from_slice(
        &fs::read(
            root.path()
                .join("aidlc/.aidlc-sessions/session-a.binding.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        binding.get("intent").and_then(serde_json::Value::as_str),
        Some(current.trim())
    );
    assert!(
        intents
            .join(current.trim())
            .join("aidlc-state.md")
            .is_file()
    );
    let cursor = fs::read_to_string(intents.join(current.trim()).join(".aidlc-execution")).unwrap();
    let expected_uuid = cursor.lines().nth(1).unwrap();
    let ended = invoke(
        root.path(),
        "session-end",
        r#"{"session_id":"session-a","reason":"exit"}"#,
    );
    assert!(ended.status.success(), "{ended:?}");
    let audit = fs::read_dir(intents.join(current.trim()).join("audit"))
        .unwrap()
        .map(|entry| fs::read_to_string(entry.unwrap().path()).unwrap())
        .collect::<String>();
    assert_eq!(
        audit.matches("**Event**: SESSION_ENDED").count(),
        1,
        "作成したsessionの終了を正しい作業へ記録する"
    );
    assert_eq!(
        fs::read_to_string(root.path().join("aidlc/.aidlc-sessions/session-a")).unwrap(),
        format!("{expected_uuid}\n")
    );
    assert!(
        !root.path().join("aidlc/.aidlc-sessions/session-b").exists(),
        "後から開始した別会話へstampを付け替えない"
    );
}

fn local_path(path: &str, case: &serde_json::Value) -> std::path::PathBuf {
    // 測定済みのhost名だけを対称に写す。clone/session/stageの固定IDは保持する。
    let Some(encoded) = case
        .get("initial_files")
        .unwrap()
        .get("aidlc/.aidlc-clone-id")
        .and_then(serde_json::Value::as_str)
    else {
        return path.into();
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .unwrap();
    let clone =
        core_command_domain::workspace::CloneId::parse(String::from_utf8(bytes).unwrap().trim())
            .unwrap();
    if path.contains("/audit/") && path.ends_with(&format!("-{}.md", clone.as_str())) {
        let host = hostname::get().unwrap().to_string_lossy().into_owned();
        return std::path::Path::new(path)
            .with_file_name(core_command_domain::workspace::ShardName::of(&host, &clone).as_str());
    }
    path.into()
}

#[test]
fn creating_another_intent_records_the_exact_session_handoff() {
    let root = tempfile::tempdir().unwrap();
    let first = create_intent(root.path(), "first", "bugfix", Some("session-a"));
    assert!(first.status.success(), "{first:?}");
    let sessions = root.path().join("aidlc/.aidlc-sessions");
    let from = fs::read_to_string(sessions.join("session-a")).unwrap();
    assert!(!sessions.join("session-a.handoff.json").exists());
    let before = chrono::Utc::now().timestamp_millis();
    let second = create_intent(root.path(), "second", "bugfix", Some("session-a"));
    assert!(second.status.success(), "{second:?}");
    let after = chrono::Utc::now().timestamp_millis();
    let to = fs::read_to_string(sessions.join("session-a")).unwrap();
    assert_ne!(from, to);
    let handoff_text = fs::read_to_string(sessions.join("session-a.handoff.json")).unwrap();
    let handoff: serde_json::Value = serde_json::from_str(&handoff_text).unwrap();
    assert_eq!(handoff.as_object().unwrap().len(), 3);
    assert_eq!(
        handoff
            .get("fromIntentUuid")
            .and_then(serde_json::Value::as_str),
        Some(from.trim())
    );
    assert_eq!(
        handoff
            .get("toIntentUuid")
            .and_then(serde_json::Value::as_str),
        Some(to.trim())
    );
    assert!(
        (before..=after).contains(
            &handoff
                .get("issuedAtMs")
                .and_then(serde_json::Value::as_i64)
                .unwrap()
        )
    );
    assert_eq!(
        handoff_text,
        format!(
            "{{\"fromIntentUuid\":\"{}\",\"toIntentUuid\":\"{}\",\"issuedAtMs\":{}}}\n",
            from.trim(),
            to.trim(),
            handoff.get("issuedAtMs").unwrap()
        )
    );
    let layout = aidlc::layout::Layout::resolve(root.path());
    assert!(layout.has_current_handoff("session-a", chrono::Utc::now()));
}

#[test]
fn rejected_creation_keeps_the_existing_session_attribution_unchanged() {
    let root = tempfile::tempdir().unwrap();
    let created = create_intent(root.path(), "first", "bugfix", Some("session-a"));
    assert!(created.status.success(), "{created:?}");
    let sessions = root.path().join("aidlc/.aidlc-sessions");
    let stamp = fs::read(sessions.join("session-a")).unwrap();
    let binding = fs::read(sessions.join("session-a.binding.json")).unwrap();
    let rejected = create_intent(root.path(), "invalid", "no-such-scope", Some("session-a"));
    assert!(!rejected.status.success(), "{rejected:?}");
    assert_eq!(fs::read(sessions.join("session-a")).unwrap(), stamp);
    assert_eq!(
        fs::read(sessions.join("session-a.binding.json")).unwrap(),
        binding
    );
    assert!(!sessions.join("session-a.handoff.json").exists());
}

#[test]
fn creation_without_a_valid_session_does_not_claim_the_last_started_conversation() {
    for session in [None, Some("../invalid-session")] {
        let root = tempfile::tempdir().unwrap();
        let started = invoke(
            root.path(),
            "session-start",
            r#"{"source":"startup","session_id":"session-a"}"#,
        );
        assert!(started.status.success(), "{started:?}");
        let sessions = root.path().join("aidlc/.aidlc-sessions");
        let binding = fs::read(sessions.join("session-a.binding.json")).unwrap();
        let created = create_intent(root.path(), "anonymous", "bugfix", session);
        assert!(created.status.success(), "{created:?}");
        assert_eq!(
            fs::read(sessions.join("session-a.binding.json")).unwrap(),
            binding
        );
        assert!(!sessions.join("session-a").exists());
        assert!(!sessions.join("session-a.handoff.json").exists());
    }
}
fn normalize_clock(text: &str) -> String {
    text.split_inclusive('\n')
        .map(|line| {
            if let Some(clock) = line.strip_prefix("**Timestamp**: ") {
                let value = clock.strip_suffix('\n').unwrap_or(clock);
                assert_eq!(value.len(), 20);
                assert!(value.ends_with('Z'));
                assert!(chrono::DateTime::parse_from_rfc3339(value).is_ok());
                if line.ends_with('\n') {
                    "**Timestamp**: <TIME>\n".to_string()
                } else {
                    "**Timestamp**: <TIME>".to_string()
                }
            } else {
                line.to_string()
            }
        })
        .collect()
}

#[test]
fn session_start_sources_keep_context_and_audit_mapping_distinct() {
    for id in [
        "active/startup",
        "active/resume",
        "active/clear",
        "active/compact",
        "active/unknown",
        "active/malformed-start",
        "active/empty-start",
        "active/null-start",
    ] {
        assert_audit_case(id, "session-start");
    }
}

#[test]
fn precompact_records_validation_without_changing_the_workflow() {
    assert_audit_case("active/compact-valid", "validate-state");
}

#[test]
fn precompact_warns_about_missing_sections_and_records_invalid_state() {
    assert_audit_case("active/compact-invalid", "validate-state");
}

#[test]
fn cold_precompact_only_creates_the_health_observation() {
    let observation = case("cold/compact");
    let root = tempfile::tempdir().unwrap();
    restore(root.path(), &observation);
    let output = invoke(
        root.path(),
        "validate-state",
        r#"{"session_id":"session-a"}"#,
    );
    assert!(output.status.success() && output.stdout.is_empty() && output.stderr.is_empty());
    let heartbeat = root
        .path()
        .join("aidlc/spaces/default/intents/.aidlc-hooks-health/validate-state.last");
    assert!(chrono::DateTime::parse_from_rfc3339(&fs::read_to_string(heartbeat).unwrap()).is_ok());
    assert!(
        !root
            .path()
            .join("aidlc/spaces/default/intents/active-intent")
            .exists()
    );
    assert!(
        !root
            .path()
            .join("aidlc/spaces/default/intents/.aidlc-recovery.md")
            .exists()
    );
    assert!(
        !root
            .path()
            .join("aidlc/spaces/default/intents/.aidlc-store.sqlite")
            .exists()
    );
}

#[test]
fn precompact_without_the_current_shard_keeps_audit_absent_and_writes_breadcrumb() {
    let observation = case("active/compact-valid");
    let root = tempfile::tempdir().unwrap();
    restore(root.path(), &observation);
    let relative = observation
        .get("initial_files")
        .unwrap()
        .as_object()
        .unwrap()
        .keys()
        .find(|path| path.contains("/audit/") && path.ends_with(".md"))
        .unwrap();
    let audit = root.path().join(local_path(relative, &observation));
    fs::remove_file(&audit).unwrap();
    let output = invoke(root.path(), "validate-state", "malformed");
    assert!(
        output.status.success() && output.stdout.is_empty() && output.stderr.is_empty(),
        "{output:?}"
    );
    assert!(!audit.exists());
    assert!(
        audit
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(".aidlc-recovery.md")
            .exists()
    );
    assert!(
        !root
            .path()
            .join("aidlc/spaces/default/intents/.aidlc-store.sqlite")
            .exists()
    );
}

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;

#[path = "../../../../tests/support/tool_link.rs"]
mod tool_link;
