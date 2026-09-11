//! 実際の指示発行とPreCompactを、公開CLI・SQLite・投影ファイルで接続する。
#![allow(clippy::unwrap_used)]
use std::{
    fs,
    io::Write as _,
    path::Path,
    process::{Command, Output, Stdio},
};

fn command(root: &Path, binary: &Path, args: &[&str], input: &str) -> Output {
    let mut child = Command::new(binary)
        .args(args)
        .current_dir(root)
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", root)
        .env("PATH", "/usr/bin:/bin")
        .env("AIDLC_SESSION_OVERRIDE", "session-a")
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

fn issued_workspace() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let data = root.path().join(".claude/tools/data");
    fs::create_dir_all(&data).unwrap();
    for name in ["stage-graph.json", "scope-grid.json", "harness.json"] {
        fs::copy(
            repo.join("tests/golden/upstream-a277af21/data").join(name),
            data.join(name),
        )
        .unwrap();
    }
    fs::create_dir_all(root.path().join(".claude/scopes")).unwrap();
    fs::copy(
        repo.join(".claude/scopes/aidlc-bugfix.md"),
        root.path().join(".claude/scopes/aidlc-bugfix.md"),
    )
    .unwrap();
    let utility = root.path().join("aidlc-utility");
    tool_link::link_tool(&utility).unwrap();
    let created = command(
        root.path(),
        &utility,
        &[
            "intent-create",
            "--scope",
            "bugfix",
            "--label",
            "compaction",
            "--arguments",
            "Verify session compaction",
        ],
        "",
    );
    assert!(created.status.success(), "{created:?}");
    let issued = command(
        root.path(),
        Path::new(env!("CARGO_BIN_EXE_aidlc")),
        &["next"],
        "",
    );
    assert!(issued.status.success(), "{issued:?}");
    root
}

fn record(root: &Path) -> std::path::PathBuf {
    let intents = root.join("aidlc/spaces/default/intents");
    intents.join(
        fs::read_to_string(intents.join("active-intent"))
            .unwrap()
            .trim(),
    )
}

async fn assign_saved_owner(root: &Path) {
    use core_command_domain::orchestration::{
        ActiveDirective, DirectiveIssued, DirectivePublication, IntentExecutionEvent,
        IntentExecutionEventId, IntentExecutionId,
    };
    use core_command_domain::workspace::{SpaceName, StorePath};
    use core_command_use_case::orchestration::IntentExecutionRepository;
    let directory = record(root);
    let cursor = fs::read_to_string(directory.join(".aidlc-execution")).unwrap();
    let id = IntentExecutionId::parse(cursor.lines().next().unwrap()).unwrap();
    let store = StorePath::for_space(&root.join("aidlc"), &SpaceName::parse("default").unwrap());
    let mut repository =
        core_command_interface_adapter::orchestration::IntentExecutionRepositoryImpl::open(&store)
            .unwrap();
    let mut execution = repository.find_by_id(&id).await.unwrap();
    let previous = execution.active_directive().unwrap();
    let publication = DirectivePublication::new(
        previous.project_sha256().to_string(),
        previous.state_sha256().to_string(),
        previous.directive().clone(),
    )
    .with_source_floor(previous.source_floor().map(str::to_string));
    // 他ハーネス等から引き継いだ保存済みownerを、公開Repositoryで用意する合成fixture。
    // 通常Claudeのnext自身に所有権を取得する挙動は追加しない。
    let owned = ActiveDirective::new(
        previous.revision() + 1,
        previous.intent_id().clone(),
        publication,
        previous.initial_state_sha256().to_string(),
        "session-a".into(),
        1,
        0,
        1,
    );
    let event = IntentExecutionEvent::DirectiveIssued(DirectiveIssued::new(
        IntentExecutionEventId::generate(),
        id,
        owned,
    ));
    execution.apply_event(execution.seq_nr() + 1, chrono::Utc::now(), &event);
    repository.store(&event, &execution).await.unwrap();
    let output = command(root, Path::new(env!("CARGO_BIN_EXE_aidlc")), &["next"], "");
    assert!(output.status.success(), "{output:?}");
}

#[tokio::test]
async fn owning_session_compaction_invalidates_the_persisted_directive_context() {
    let root = issued_workspace();
    assign_saved_owner(root.path()).await;
    let record = record(root.path());
    let marker = record.join(".aidlc-active-directive.json");
    let before: serde_json::Value = serde_json::from_slice(&fs::read(&marker).unwrap()).unwrap();
    let state = fs::read(record.join("aidlc-state.md")).unwrap();
    let compacted = command(
        root.path(),
        Path::new(env!("CARGO_BIN_EXE_aidlc")),
        &["hook", "validate-state"],
        r#"{"session_id":"session-a"}"#,
    );
    assert!(compacted.status.success(), "{compacted:?}");
    let after: serde_json::Value = serde_json::from_slice(&fs::read(&marker).unwrap()).unwrap();
    assert_eq!(
        after.get("kind").and_then(serde_json::Value::as_str),
        Some("error")
    );
    assert_eq!(
        after
            .get("owner_session")
            .and_then(serde_json::Value::as_str),
        Some("session-a")
    );
    assert_eq!(
        after
            .get("context_epoch")
            .and_then(serde_json::Value::as_u64),
        Some(
            before
                .get("context_epoch")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
                + 1
        )
    );
    assert_eq!(fs::read(record.join("aidlc-state.md")).unwrap(), state);
}

#[test]
fn ordinary_claude_sessionless_directive_is_not_claimed_by_precompact() {
    let root = issued_workspace();
    let marker = record(root.path()).join(".aidlc-active-directive.json");
    let before = fs::read(&marker).unwrap();
    let output = command(
        root.path(),
        Path::new(env!("CARGO_BIN_EXE_aidlc")),
        &["hook", "validate-state"],
        r#"{"session_id":"session-a"}"#,
    );
    assert!(output.status.success(), "{output:?}");
    assert_eq!(fs::read(&marker).unwrap(), before);
}

#[tokio::test]
async fn missing_malformed_and_foreign_sessions_preserve_the_owned_directive() {
    let root = issued_workspace();
    assign_saved_owner(root.path()).await;
    let marker = record(root.path()).join(".aidlc-active-directive.json");
    let before = fs::read(&marker).unwrap();
    for input in [
        "",
        "{",
        "null",
        "{}",
        r#"{"session_id":"session-b"}"#,
        r#"{"session_id":"../session-a"}"#,
    ] {
        let output = command(
            root.path(),
            Path::new(env!("CARGO_BIN_EXE_aidlc")),
            &["hook", "validate-state"],
            input,
        );
        assert!(output.status.success(), "{input}: {output:?}");
        assert_eq!(fs::read(&marker).unwrap(), before, "{input}");
    }
}

#[tokio::test]
async fn compaction_with_changed_state_cannot_invalidate_the_original_context() {
    let root = issued_workspace();
    assign_saved_owner(root.path()).await;
    let directory = record(root.path());
    let marker = directory.join(".aidlc-active-directive.json");
    let before = fs::read(&marker).unwrap();
    let state_path = directory.join("aidlc-state.md");
    let state = format!(
        "{}\n<!-- changed observation -->\n",
        fs::read_to_string(&state_path).unwrap()
    );
    fs::write(&state_path, &state).unwrap();
    let output = command(
        root.path(),
        Path::new(env!("CARGO_BIN_EXE_aidlc")),
        &["hook", "validate-state"],
        r#"{"sessionId":"session-a"}"#,
    );
    assert!(output.status.success(), "{output:?}");
    assert_eq!(fs::read(&marker).unwrap(), before);
    assert_eq!(fs::read_to_string(state_path).unwrap(), state);
}

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;

#[path = "../../../../tests/support/tool_link.rs"]
mod tool_link;
