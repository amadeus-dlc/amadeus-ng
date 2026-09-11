//! 通常rootのjumpを公開CLI・実SQLite・再投影で検証する。
#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};
struct Workspace {
    temp: tempfile::TempDir,
}

/// 固定本家 2.7.1 の直接 execute 観測 (`tests/golden/selfhost-stage1/jump-direct-execute.json`
/// — 採取元は intent 記録の `jump-logs/questions-observations.json`) を 1 件引く。
fn upstream_observation(case: &str) -> serde_json::Value {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/jump-direct-execute.json"
    ))
    .unwrap();
    corpus
        .get("observations")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row.get("case").unwrap() == case && row.get("mode").unwrap() == "upstream")
        .unwrap()
        .clone()
}

/// 観測どおりの引数で `aidlc-jump` を打ち、timestamp を除いた stdout を本家と全文比較する。
fn assert_execute_matches_upstream(workspace: &Workspace, observed: &serde_json::Value) {
    let arguments = observed
        .get("args")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<Vec<_>>();
    let output = workspace.run("aidlc-jump", &arguments);
    assert!(output.status.success(), "{output:?}");
    let mut actual: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let mut expected: serde_json::Value =
        serde_json::from_str(observed.get("stdout").unwrap().as_str().unwrap()).unwrap();
    actual.as_object_mut().unwrap().remove("timestamp");
    expected.as_object_mut().unwrap().remove("timestamp");
    assert_eq!(actual, expected);
}

/// 本家は初期化フェーズへの直接 backward を受理し、初期化 3 段と現在 stage を pending へ戻す
/// (`initialization-target`)。resolve の初期化拒否は execute には及ばない。
#[test]
fn an_initialization_target_is_executed_directly_and_resets_the_initialization_stages() {
    let observed = upstream_observation("initialization-target");
    let workspace = Workspace::new();
    assert_execute_matches_upstream(&workspace, &observed);
    let state = fs::read_to_string(workspace.record().join("aidlc-state.md")).unwrap();
    assert!(
        state.contains("- **Current Stage**: workspace-scaffold\n"),
        "{state}"
    );
    assert!(state.contains("- [-] workspace-scaffold"), "{state}");
    assert!(state.contains("- [ ] workspace-detection"), "{state}");
    assert!(state.contains("- [ ] state-init"), "{state}");
    assert!(state.contains("- [ ] reverse-engineering"), "{state}");
}

/// 本家は `--scope classic` を渡すと classic の静的な列で到達可否と読み飛ばしを導く
/// (`foreign-scope`)。state の Scope は変わらず、classic で EXECUTE の介在 pending が `[S]` になる。
#[test]
fn a_foreign_scope_execute_derives_skips_from_that_scopes_static_plan() {
    let observed = upstream_observation("foreign-scope");
    let workspace = Workspace::new();
    assert_execute_matches_upstream(&workspace, &observed);
    let state = fs::read_to_string(workspace.record().join("aidlc-state.md")).unwrap();
    assert!(state.contains("- **Scope**: bugfix\n"), "{state}");
    assert!(
        state.contains("- **Current Stage**: domain-design\n"),
        "{state}"
    );
    for line in [
        "- [S] reverse-engineering",
        "- [S] practices-discovery",
        "- [S] requirements-analysis",
        "- [S] user-stories",
        "- [S] refined-mockups",
        "- [-] domain-design",
    ] {
        assert!(state.contains(line), "{line} が無い:\n{state}");
    }
}

#[test]
fn an_explicit_backward_direction_is_not_rederived_from_the_target() {
    let observed = upstream_observation("direction-mismatch");
    let observed = &observed;
    let workspace = Workspace::new();
    let arguments = observed
        .get("args")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<Vec<_>>();
    let output = workspace.run("aidlc-jump", &arguments);
    assert!(output.status.success(), "{output:?}");
    let mut actual: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let mut expected: serde_json::Value =
        serde_json::from_str(observed.get("stdout").unwrap().as_str().unwrap()).unwrap();
    actual.as_object_mut().unwrap().remove("timestamp");
    expected.as_object_mut().unwrap().remove("timestamp");
    assert_eq!(actual, expected);
    let state = fs::read_to_string(workspace.record().join("aidlc-state.md")).unwrap();
    assert!(state.contains("- [-] reverse-engineering"));
    assert!(state.contains("- [-] requirements-analysis"));
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
        fs::write(root.join("source.rs"), "fn main() {}\n").unwrap();
        for tool in ["aidlc-utility", "aidlc-jump"] {
            tool_link::link_tool(&temp.path().join(tool)).unwrap();
        }
        let result = Self { temp };
        let start = result.run(
            "aidlc-utility",
            &[
                "intent-create",
                "--scope",
                "bugfix",
                "--label",
                "jump",
                "--arguments",
                "Fix jump",
            ],
        );
        assert!(start.status.success(), "{start:?}");
        result
    }
    fn root(&self) -> PathBuf {
        self.temp.path().join("workspace")
    }
    fn record(&self) -> PathBuf {
        let intents = self.root().join("aidlc/spaces/default/intents");
        intents.join(
            fs::read_to_string(intents.join("active-intent"))
                .unwrap()
                .trim(),
        )
    }
    fn run(&self, tool: &str, args: &[&str]) -> Output {
        Command::new(self.temp.path().join(tool))
            .args(args)
            .current_dir(self.root())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.temp.path().join("home"))
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap()
    }
    fn audit(&self) -> String {
        fs::read_dir(self.record().join("audit"))
            .unwrap()
            .map(|entry| fs::read_to_string(entry.unwrap().path()).unwrap())
            .collect()
    }
}

#[test]
fn redo_records_one_source_snapshot_for_both_transition_rows() {
    let workspace = Workspace::new();
    let before = workspace.audit();
    let output = workspace.run(
        "aidlc-jump",
        &[
            "execute",
            "--target",
            "reverse-engineering",
            "--direction",
            "redo",
        ],
    );
    assert!(output.status.success(), "{output:?}");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(*value.get("direction").unwrap(), "redo");
    assert_eq!(*value.get("target").unwrap(), "reverse-engineering");
    assert_eq!(*value.get("stages_skipped").unwrap(), serde_json::json!([]));
    assert_eq!(
        *value.get("stages_reset").unwrap(),
        serde_json::Value::Array(vec![serde_json::Value::String(
            "reverse-engineering".into()
        )])
    );
    let after = workspace.audit();
    let appended = after.strip_prefix(&before).unwrap();
    let baseline = appended
        .lines()
        .filter_map(|line| line.strip_prefix("**Source Baseline**: "))
        .collect::<Vec<_>>();
    assert_eq!(baseline.len(), 2);
    assert_eq!(baseline.first().unwrap(), baseline.get(1).unwrap());
    let hash = baseline.first().unwrap().strip_prefix("sha256:").unwrap();
    assert!(
        workspace
            .record()
            .join(format!(
                ".aidlc-source-review/code-generation/baseline-{}.tsv",
                &hash[..12]
            ))
            .is_file()
    );
}

#[test]
fn backward_jump_records_declared_targets_and_existing_downstream_reviews() {
    let workspace = Workspace::new();
    let forward = workspace.run(
        "aidlc-jump",
        &[
            "execute",
            "--target",
            "code-generation",
            "--direction",
            "forward",
        ],
    );
    assert!(forward.status.success(), "{forward:?}");
    let summary = workspace
        .record()
        .join("construction/code-generation/code-summary.md");
    fs::create_dir_all(summary.parent().unwrap()).unwrap();
    fs::write(&summary, "# Implementation\n\n## Review\nREADY\n").unwrap();
    let before = workspace.audit();
    let backward = workspace.run(
        "aidlc-jump",
        &[
            "execute",
            "--target",
            "requirements-analysis",
            "--direction",
            "backward",
        ],
    );
    assert!(backward.status.success(), "{backward:?}");
    let after = workspace.audit();
    let appended = after.strip_prefix(&before).unwrap();
    let field = |name: &str| {
        let prefix = format!("**{name}**: ");
        serde_json::from_str::<Vec<String>>(
            appended
                .lines()
                .find_map(|line| line.strip_prefix(&prefix))
                .expect("jumpの無効化監査が必要"),
        )
        .unwrap()
    };
    assert!(
        field("Changed Upstream Artifacts")
            .iter()
            .any(|path| path.ends_with("inception/requirements-analysis/requirements.md"))
    );
    let relative = summary
        .strip_prefix(workspace.root())
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(
        field("Invalidated Downstream Artifacts"),
        vec![relative.to_string()]
    );
    assert_eq!(
        field("Invalidated Downstream Reviews"),
        vec![format!("{relative}#Review")]
    );
}

#[test]
fn resolve_reads_the_projected_target_and_does_not_move_the_workflow() {
    let workspace = Workspace::new();
    let before = workspace.audit();
    let state = fs::read(workspace.record().join("aidlc-state.md")).unwrap();
    let output = workspace.run("aidlc-jump", &["resolve", "--stage", "code-generation"]);
    assert!(output.status.success(), "{output:?}");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(*value.get("target_slug").unwrap(), "code-generation");
    assert_eq!(*value.get("target_number").unwrap(), "3.5");
    assert_eq!(*value.get("current_slug").unwrap(), "reverse-engineering");
    assert_eq!(*value.get("direction").unwrap(), "forward");
    assert_eq!(
        *value.get("affected_stages").unwrap(),
        serde_json::Value::Array(vec![serde_json::Value::String(
            "requirements-analysis".into()
        )])
    );
    assert_eq!(workspace.audit(), before);
    assert_eq!(
        fs::read(workspace.record().join("aidlc-state.md")).unwrap(),
        state
    );
}

#[tokio::test]
async fn replay_preserves_the_observation_after_source_and_review_files_change() {
    use core_command_domain::workspace::{SpaceName, StorePath};
    use core_read_model_updater::{
        orchestration::{GlobalSeqNr, JournalReader, JournalReaderImpl},
        workspace::{ReadModel, ResolvedPlan, project},
    };
    let workspace = Workspace::new();
    assert!(
        workspace
            .run(
                "aidlc-jump",
                &[
                    "execute",
                    "--target",
                    "code-generation",
                    "--direction",
                    "forward"
                ]
            )
            .status
            .success()
    );
    let summary = workspace
        .record()
        .join("construction/code-generation/code-summary.md");
    fs::create_dir_all(summary.parent().unwrap()).unwrap();
    fs::write(&summary, "# Implementation\n\n## Review\nREADY\n").unwrap();
    let before_state = fs::read_to_string(workspace.record().join("aidlc-state.md")).unwrap();
    let before_audit = workspace.audit();
    assert!(
        workspace
            .run(
                "aidlc-jump",
                &[
                    "execute",
                    "--target",
                    "requirements-analysis",
                    "--direction",
                    "backward"
                ]
            )
            .status
            .success()
    );
    let after_audit = workspace.audit();
    let expected = after_audit.strip_prefix(&before_audit).unwrap();
    fs::remove_file(summary).unwrap();
    fs::write(workspace.root().join("source.rs"), "fn changed() {}\n").unwrap();
    let store = StorePath::for_space(&workspace.root().join("aidlc"), &SpaceName::default());
    let reader = JournalReaderImpl::open(&store).unwrap();
    let history = reader.events_after(GlobalSeqNr::ZERO).await.unwrap();
    let last = history.executions().last().unwrap();
    let plan = ResolvedPlan::of(history.intents().first().unwrap());
    let mut replay = ReadModel::new(before_state);
    project(std::slice::from_ref(last), &plan, &mut replay).unwrap();
    assert_eq!(replay.appended_audit(), expected);
    assert_eq!(
        replay.state(),
        fs::read_to_string(workspace.record().join("aidlc-state.md")).unwrap()
    );
}

#[test]
fn normal_jump_stdout_stderr_and_audit_match_fixed_upstream() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/jump-normal.json"
    ))
    .unwrap();
    let workspace = Workspace::new();
    let normalise = |text: &str| {
        let mut text = text
            .replace(
                workspace
                    .record()
                    .strip_prefix(workspace.root())
                    .unwrap()
                    .to_str()
                    .unwrap(),
                "<RECORD>",
            )
            .replace(workspace.root().to_str().unwrap(), "<ROOT>");
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
            && let Some(timestamp) = value.get("timestamp").and_then(serde_json::Value::as_str)
        {
            text = text.replace(timestamp, "<TS>");
        }
        text.split_inclusive('\n')
            .map(|line| {
                if line.starts_with("**Timestamp**: ") {
                    "**Timestamp**: <TS>\n"
                } else {
                    line
                }
            })
            .collect::<String>()
    };
    for step in corpus.get("observations").unwrap().as_array().unwrap() {
        let id = step.get("id").unwrap().as_str().unwrap();
        if id == "backward" {
            let summary = workspace
                .record()
                .join("construction/code-generation/code-summary.md");
            fs::create_dir_all(summary.parent().unwrap()).unwrap();
            fs::write(summary, "# Implementation\n\n## Review\nREADY\n").unwrap();
        }
        let before = workspace.audit();
        let args = step
            .get("args")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect::<Vec<_>>();
        let actual = workspace.run("aidlc-jump", &args);
        assert_eq!(
            actual.status.code().map(i64::from),
            step.get("exit").unwrap().as_i64(),
            "{id}"
        );
        assert_eq!(
            normalise(std::str::from_utf8(&actual.stdout).unwrap()),
            step.get("stdout").unwrap().as_str().unwrap(),
            "{id} stdout"
        );
        assert_eq!(
            normalise(std::str::from_utf8(&actual.stderr).unwrap()),
            step.get("stderr").unwrap().as_str().unwrap(),
            "{id} stderr"
        );
        let after = workspace.audit();
        assert_eq!(
            normalise(after.strip_prefix(&before).unwrap()),
            step.get("audit").unwrap().as_str().unwrap(),
            "{id} audit"
        );
    }
}

#[tokio::test]
async fn several_unpublished_jumps_can_share_one_source_snapshot() {
    use core_command_domain::{
        orchestration::{IntentExecutionEventId, JumpObservation, SourceBaseline},
        workflow_definition::StageSlug,
        workspace::{SpaceName, StorePath},
    };
    use core_command_interface_adapter::orchestration::{
        IntentExecutionRepositoryImpl, IntentRepositoryImpl,
    };
    use core_command_use_case::orchestration::JumpUseCase;
    let workspace = Workspace::new();
    let cursor = aidlc::execution_cursor::ExecutionCursor::read(&workspace.record())
        .unwrap()
        .unwrap();
    let store = StorePath::for_space(&workspace.root().join("aidlc"), &SpaceName::default());
    let mut command = JumpUseCase::new(
        IntentExecutionRepositoryImpl::open(&store).unwrap(),
        IntentRepositoryImpl::open(&store).unwrap(),
        core_command_interface_adapter::orchestration::WorkflowDefinitionRepositoryImpl::open(
            &store,
        )
        .unwrap(),
    );
    for _ in 0..2 {
        command
            .execute(
                cursor.execution_id(),
                &IntentExecutionEventId::generate(),
                &StageSlug::parse("reverse-engineering").unwrap(),
                core_command_domain::orchestration::JumpDirection::Redo,
                None,
                JumpObservation::new(
                    SourceBaseline::new(Some(String::new())).unwrap(),
                    Vec::new(),
                ),
                chrono::Utc::now(),
            )
            .await
            .unwrap();
    }
    let published = workspace.run("aidlc-jump", &["resolve", "--stage", "code-generation"]);
    assert!(published.status.success(), "{published:?}");
    let path = workspace
        .record()
        .join(".aidlc-source-review/code-generation/baseline-e3b0c44298fc.tsv");
    assert_eq!(fs::read(path).unwrap(), b"");
    assert_eq!(
        workspace.audit().matches("**Event**: STAGE_JUMPED").count(),
        2
    );
}

#[test]
fn the_previous_read_schema_is_rebuilt_without_changing_the_journal() {
    use core_command_domain::workspace::{SpaceName, StorePath};
    let workspace = Workspace::new();
    let store = StorePath::for_space(&workspace.root().join("aidlc"), &SpaceName::default());
    let connection = rusqlite::Connection::open(store.as_path()).unwrap();
    let count = || {
        connection
            .query_row("SELECT COUNT(*) FROM journal", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap()
    };
    let before = count();
    connection
        .execute_batch("ALTER TABLE read_next_jump DROP COLUMN resolution; PRAGMA user_version=4;")
        .unwrap();
    let output = workspace.run("aidlc-jump", &["resolve", "--stage", "code-generation"]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(count(), before);
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value.get("target_slug").unwrap().as_str(),
        Some("code-generation")
    );
}

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;

#[path = "../../../../tests/support/tool_link.rs"]
mod tool_link;
