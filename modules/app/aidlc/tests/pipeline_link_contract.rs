//! 固定本家のlog linkを、隔離した一時workspaceでプロセス境界から比較する。
#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};
struct Workspace {
    temp: tempfile::TempDir,
}
impl Workspace {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        fs::create_dir(&root).unwrap();
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
        for tool in ["aidlc-utility", "aidlc-log"] {
            tool_link::link_tool(&temp.path().join(tool)).unwrap();
        }
        fs::create_dir_all(root.join("aidlc/spaces/default/intents")).unwrap();
        Self { temp }
    }
    fn root(&self) -> PathBuf {
        self.temp.path().join("workspace")
    }
    fn run(&self, tool: &str, args: &[String]) -> Output {
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
    fn record(&self) -> Option<PathBuf> {
        let intents = self.root().join("aidlc/spaces/default/intents");
        fs::read_to_string(intents.join("active-intent"))
            .ok()
            .map(|name| intents.join(name.trim()))
    }
    fn audit(&self) -> String {
        let Some(record) = self.record() else {
            return String::new();
        };
        let mut paths = fs::read_dir(record.join("audit"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect::<Vec<_>>();
        paths.sort();
        paths
            .into_iter()
            .map(|p| fs::read_to_string(p).unwrap())
            .collect()
    }
    fn normalise(&self, text: &str) -> String {
        let root = self.root();
        let mut text = text.to_string();
        if let Some(record) = self.record() {
            text = text.replace(
                record.strip_prefix(&root).unwrap().to_str().unwrap(),
                "<RECORD>",
            );
        }
        text = text.replace(root.to_str().unwrap(), "<ROOT>");
        // macOS では一時ディレクトリの realpath に /private が付く。Linux と揃えるため両側で落とす。
        text = text.replace("/private<ROOT>", "<ROOT>");
        text.split_inclusive('\n')
            .map(|line| {
                if line.starts_with("**Timestamp**: ") {
                    "**Timestamp**: <TS>\n"
                } else {
                    line
                }
            })
            .collect()
    }
    fn handoff(&self, kind: &str) -> PathBuf {
        let path = self
            .record()
            .unwrap()
            .join("inception/reverse-engineering/developer-scan.md");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        if path.is_dir() {
            fs::remove_dir_all(&path).unwrap();
        } else if fs::symlink_metadata(&path).is_ok() {
            fs::remove_file(&path).unwrap();
        }
        match kind {
            "directory" => fs::create_dir(&path).unwrap(),
            "symlink" => {
                fs::write(self.root().join("real.md"), "handoff\n").unwrap();
                #[cfg(unix)]
                std::os::unix::fs::symlink(self.root().join("real.md"), &path).unwrap();
            }
            _ => {
                fs::write(
                    &path,
                    if kind == "changed" {
                        "changed handoff\n"
                    } else {
                        "handoff\n"
                    },
                )
                .unwrap();
                let seconds = match kind {
                    "old" => 946_684_800,
                    "rewritten" => 4_102_444_801,
                    _ => 4_102_444_800,
                };
                let at = std::time::UNIX_EPOCH + std::time::Duration::from_secs(seconds);
                fs::File::open(&path)
                    .unwrap()
                    .set_times(fs::FileTimes::new().set_accessed(at).set_modified(at))
                    .unwrap();
            }
        }
        path
    }
}
fn text<'a>(value: &'a serde_json::Value, key: &str) -> &'a str {
    value.get(key).unwrap().as_str().unwrap()
}
#[test]
fn public_link_inputs_and_results_match_fixed_upstream() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/pipeline-link.json"
    ))
    .unwrap();
    for scenario in corpus.get("observations").unwrap().as_array().unwrap() {
        let workspace = Workspace::new();
        if !scenario.get("cold").unwrap().as_bool().unwrap() {
            fs::write(workspace.root().join("source.rs"), "fn main() {}\n").unwrap();
            let args = [
                "intent-create",
                "--scope",
                "bugfix",
                "--label",
                "link",
                "--arguments",
                "Fix pipeline link",
            ]
            .map(str::to_string);
            let output = workspace.run("aidlc-utility", &args);
            assert!(output.status.success(), "{output:?}");
        }
        for step in scenario.get("steps").unwrap().as_array().unwrap() {
            if let Some(kind) = step.get("file").and_then(serde_json::Value::as_str) {
                workspace.handoff(kind);
            }
            let handoff = workspace
                .record()
                .unwrap_or_default()
                .join("inception/reverse-engineering/developer-scan.md");
            let args = step
                .get("args")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|v| {
                    if v.as_str() == Some("<HANDOFF>") {
                        handoff
                            .strip_prefix(workspace.root())
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string()
                    } else {
                        v.as_str().unwrap().to_string()
                    }
                })
                .collect::<Vec<_>>();
            let before_state = workspace
                .record()
                .map(|record| fs::read(record.join("aidlc-state.md")).unwrap());
            let before = workspace.audit();
            let output = workspace.run("aidlc-log", &args);
            let label = format!("{}/{}", text(scenario, "id"), text(step, "id"));
            assert_eq!(
                output.status.code(),
                Some(i32::try_from(step.get("exit").unwrap().as_i64().unwrap()).unwrap()),
                "{label}"
            );
            assert_eq!(
                workspace.normalise(&String::from_utf8(output.stdout).unwrap()),
                text(step, "stdout").replace("/private<ROOT>", "<ROOT>"),
                "{label}: stdout"
            );
            assert_eq!(
                workspace.normalise(&String::from_utf8(output.stderr).unwrap()),
                text(step, "stderr").replace("/private<ROOT>", "<ROOT>"),
                "{label}: stderr"
            );
            let after_state = workspace
                .record()
                .map(|record| fs::read(record.join("aidlc-state.md")).unwrap());
            assert_eq!(
                before_state != after_state,
                step.get("state_changed").unwrap().as_bool().unwrap(),
                "{label}: state effect"
            );
            let after = workspace.audit();
            assert_eq!(
                workspace.normalise(after.strip_prefix(&before).unwrap()),
                text(step, "audit"),
                "{label}: audit"
            );
        }
    }
}

impl Workspace {
    fn start(&self) {
        fs::write(self.root().join("source.rs"), "fn main() {}\n").unwrap();
        let output = self.run(
            "aidlc-utility",
            &[
                "intent-create",
                "--scope",
                "bugfix",
                "--label",
                "link",
                "--arguments",
                "Fix pipeline link",
            ]
            .map(str::to_string),
        );
        assert!(output.status.success(), "{output:?}");
    }
    fn link(&self, agent: &str, single: bool) -> Output {
        let handoff = self
            .record()
            .unwrap()
            .join("inception/reverse-engineering/developer-scan.md");
        let mut args = vec![
            "link".into(),
            "--stage".into(),
            "reverse-engineering".into(),
            "--link".into(),
            agent.into(),
            "--artifact".into(),
            handoff
                .strip_prefix(self.root())
                .unwrap()
                .to_str()
                .unwrap()
                .into(),
        ];
        if single {
            args.push("--single".into());
        }
        self.run("aidlc-log", &args)
    }
    async fn boundary(&self, kind: &str) {
        use core_command_domain::workspace::{SpaceName, StorePath};
        use core_command_interface_adapter::orchestration::{
            IntentExecutionRepositoryImpl, IntentRepositoryImpl,
        };
        use core_command_use_case::orchestration::{
            IntentExecutionRepository as _, IntentRepository as _,
        };
        let cursor = aidlc::execution_cursor::ExecutionCursor::read(&self.record().unwrap())
            .unwrap()
            .unwrap();
        let store = StorePath::for_space(&self.root().join("aidlc"), &SpaceName::default());
        let mut repository = IntentExecutionRepositoryImpl::open(&store).unwrap();
        let intent = IntentRepositoryImpl::open(&store)
            .unwrap()
            .find_by_id(cursor.intent_id())
            .await
            .unwrap();
        let mut execution = repository.find_by_id(cursor.execution_id()).await.unwrap();
        let at = chrono::Utc::now();
        let event = match kind {
            "jump" => execution
                .jump(
                    &intent,
                    execution.cursor(),
                    core_command_domain::orchestration::IntentExecutionEventId::generate(),
                    core_command_domain::orchestration::JumpDirection::of(
                        execution.cursor().to_usize(),
                        (execution.cursor()).to_usize(),
                    ),
                    None,
                    None,
                    at,
                )
                .unwrap(),
            "reject" => execution
                .reject_gate(&intent, Some("test boundary".into()), at)
                .unwrap(),
            _ => execution
                .begin_single_stage_run(
                    &intent,
                    &core_command_domain::workflow_definition::StageSlug::parse(
                        "reverse-engineering",
                    )
                    .unwrap(),
                    at,
                )
                .unwrap(),
        };
        repository.store(&event, &execution).await.unwrap();
    }
}
#[tokio::test]
async fn a_new_attempt_rejects_old_links_and_requires_a_rewritten_handoff() {
    for boundary in ["jump", "reject"] {
        let workspace = Workspace::new();
        workspace.start();
        workspace.handoff("fresh");
        assert!(
            workspace
                .link("aidlc-developer-agent", false)
                .status
                .success()
        );
        assert!(
            workspace
                .link("aidlc-architect-agent", false)
                .status
                .success()
        );
        workspace.boundary(boundary).await;
        let later = workspace.link("aidlc-architect-agent", false);
        assert_eq!(later.status.code(), Some(1));
        assert!(
            String::from_utf8(later.stderr)
                .unwrap()
                .contains("is out of order")
        );
        let same = workspace.link("aidlc-developer-agent", false);
        assert_eq!(same.status.code(), Some(1));
        assert!(
            String::from_utf8(same.stderr)
                .unwrap()
                .contains("was not rewritten after its prior pipeline receipt")
        );
        workspace.handoff("rewritten");
        let rewritten = workspace.link("aidlc-developer-agent", false);
        assert!(rewritten.status.success(), "{rewritten:?}");
    }
}
#[tokio::test]
async fn single_receipts_do_not_replace_the_main_pipeline_chain() {
    let workspace = Workspace::new();
    workspace.start();
    workspace.handoff("fresh");
    assert!(
        workspace
            .link("aidlc-developer-agent", false)
            .status
            .success()
    );
    workspace.boundary("single").await;
    assert!(
        workspace
            .link("aidlc-developer-agent", true)
            .status
            .success()
    );
    assert!(
        workspace
            .link("aidlc-architect-agent", true)
            .status
            .success()
    );
    assert!(
        workspace
            .link("aidlc-architect-agent", false)
            .status
            .success()
    );
    let duplicate = workspace.link("aidlc-developer-agent", false);
    assert_eq!(duplicate.status.code(), Some(1));
    assert!(
        String::from_utf8(duplicate.stderr)
            .unwrap()
            .contains("already completed this attempt")
    );
    assert_eq!(
        workspace
            .audit()
            .matches("**Event**: PIPELINE_LINK_COMPLETED")
            .count(),
        4
    );
}
#[test]
fn concurrent_duplicate_completions_persist_only_one_receipt() {
    let workspace = Workspace::new();
    workspace.start();
    workspace.handoff("fresh");
    let results = std::thread::scope(|scope| {
        let first = scope.spawn(|| workspace.link("aidlc-developer-agent", false));
        let second = scope.spawn(|| workspace.link("aidlc-developer-agent", false));
        [first.join().unwrap(), second.join().unwrap()]
    });
    assert_eq!(
        results
            .iter()
            .filter(|output| output.status.success())
            .count(),
        1,
        "{results:?}"
    );
    let rejected = results
        .iter()
        .find(|output| !output.status.success())
        .expect("one of the two concurrent completions is rejected");
    assert_eq!(rejected.status.code(), Some(1), "{rejected:?}");
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("already completed this attempt"),
        "{rejected:?}"
    );
    assert_eq!(
        workspace
            .audit()
            .matches("**Event**: PIPELINE_LINK_COMPLETED")
            .count(),
        1
    );
}
#[test]
fn a_publication_failure_preserves_the_receipt_for_recovery() {
    let workspace = Workspace::new();
    workspace.start();
    workspace.handoff("fresh");
    let audit = fs::read_dir(workspace.record().unwrap().join("audit"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let before = fs::read(&audit).unwrap();
    fs::remove_file(&audit).unwrap();
    fs::create_dir(&audit).unwrap();
    let failed = workspace.link("aidlc-developer-agent", false);
    assert_eq!(failed.status.code(), Some(1));
    fs::remove_dir(&audit).unwrap();
    fs::write(&audit, before).unwrap();
    let resumed = workspace.link("aidlc-architect-agent", false);
    assert!(resumed.status.success(), "{resumed:?}");
    assert_eq!(
        workspace
            .audit()
            .matches("**Event**: PIPELINE_LINK_COMPLETED")
            .count(),
        2
    );
    let duplicate = workspace.link("aidlc-developer-agent", false);
    assert_eq!(duplicate.status.code(), Some(1));
    assert_eq!(
        workspace
            .audit()
            .matches("**Event**: PIPELINE_LINK_COMPLETED")
            .count(),
        2
    );
}

impl Workspace {
    fn engine(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .args(args)
            .current_dir(self.root())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.temp.path().join("home"))
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap()
    }
    fn next_completed(&self) -> serde_json::Value {
        let output = self.engine(&["next", "--resume"]);
        assert!(output.status.success(), "{output:?}");
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        value
            .get("pipeline")
            .expect("run-stageのpipeline参照投影が必要")
            .get("completed")
            .unwrap()
            .clone()
    }
}
#[test]
fn next_resumes_from_current_receipts_and_file_changes_expire_the_chain() {
    let workspace = Workspace::new();
    workspace.start();
    workspace.handoff("fresh");
    assert_eq!(workspace.next_completed(), serde_json::Value::Array(vec![]));
    assert!(
        workspace
            .link("aidlc-developer-agent", false)
            .status
            .success()
    );
    assert_eq!(
        workspace.next_completed(),
        serde_json::Value::Array(vec![serde_json::Value::String(
            "aidlc-developer-agent".into()
        )])
    );
    assert!(
        workspace
            .link("aidlc-architect-agent", false)
            .status
            .success()
    );
    assert_eq!(
        workspace.next_completed(),
        serde_json::Value::Array(vec![
            serde_json::Value::String("aidlc-developer-agent".into()),
            serde_json::Value::String("aidlc-architect-agent".into())
        ])
    );
    workspace.handoff("changed");
    assert_eq!(workspace.next_completed(), serde_json::Value::Array(vec![]));
}

#[test]
fn a_failed_pipeline_projection_preserves_the_prior_query_result_until_recovery() {
    use core_command_domain::workspace::{SpaceName, StorePath};
    use core_query_interface_adapter::ReadModelDaos;
    use core_query_use_case::orchestration::PipelineProgressUseCase;

    let workspace = Workspace::new();
    workspace.start();
    workspace.handoff("fresh");
    assert!(
        workspace
            .link("aidlc-developer-agent", false)
            .status
            .success()
    );
    let before = workspace.next_completed();
    let cursor = aidlc::execution_cursor::ExecutionCursor::read(&workspace.record().unwrap())
        .unwrap()
        .unwrap();
    let store = StorePath::for_space(&workspace.root().join("aidlc"), &SpaceName::default());
    let db = rusqlite::Connection::open(store.as_path()).unwrap();
    let query = PipelineProgressUseCase::new(
        ReadModelDaos::open(store.as_path())
            .unwrap()
            .pipeline_progress(),
    );
    let read = || {
        let row = query
            .execute(cursor.execution_id().as_str(), "reverse-engineering", false)
            .unwrap()
            .unwrap();
        serde_json::from_str::<serde_json::Value>(row.completed()).unwrap()
    };
    assert_eq!(read(), before);
    assert!(
        query
            .execute("other-execution", "reverse-engineering", false)
            .unwrap()
            .is_none()
    );
    assert!(
        query
            .execute(cursor.execution_id().as_str(), "other-stage", false)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        query
            .execute(cursor.execution_id().as_str(), "reverse-engineering", true)
            .unwrap()
            .unwrap()
            .completed(),
        "[]"
    );

    // SQLite障害だけを注入する。Repository/RMU/Queryは本番の実装を使う。
    db.execute_batch("CREATE TRIGGER fail_pipeline_projection BEFORE INSERT ON read_pipeline_progress BEGIN SELECT RAISE(ABORT, 'pipeline projection unavailable'); END;").unwrap();
    workspace.handoff("changed");
    let failed = workspace.engine(&["next", "--resume"]);
    let refusal: serde_json::Value = serde_json::from_slice(&failed.stdout).unwrap();
    assert_eq!(
        refusal.get("kind").and_then(serde_json::Value::as_str),
        Some("error"),
        "{refusal:?}"
    );
    assert_eq!(
        read(),
        before,
        "失敗したDELETE/INSERTは以前の投影を消さない"
    );

    db.execute_batch("DROP TRIGGER fail_pipeline_projection;")
        .unwrap();
    assert_eq!(workspace.next_completed(), serde_json::json!([]));
    assert_eq!(read(), serde_json::json!([]));
}
#[test]
fn reports_require_the_complete_current_pipeline_before_opening_or_approving_a_gate() {
    let workspace = Workspace::new();
    workspace.start();
    workspace.handoff("fresh");
    for verdict in ["awaiting-approval", "approved"] {
        let output = workspace.engine(&[
            "report",
            "--stage",
            "reverse-engineering",
            "--result",
            verdict,
            "--user-input",
            "Approve",
        ]);
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            value.get("kind").and_then(serde_json::Value::as_str),
            Some("error"),
            "{value:?}"
        );
        assert!(
            value
                .get("message")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("pipeline handoffs have not been recorded"),
            "{value:?}"
        );
    }
}

/// pipeline link の不足を告げる拒否文言は、綴りも案内文も 2.8.2 と一致する。
///
/// 2.8.2（`.claude/tools/aidlc-orchestrate.ts:7918-7931`）はこの場面で
/// (a) `aidlc engine orchestrate next` の再実行を促し、(b) `--repo <repo>` を登録 repo が
/// あるときだけ足し、(c) 末尾を「再スタンプ・検査無効化の禁止」で締める。呼び方だけでなく
/// 案内文の内容も 2.8.2 に合わせる（`order.md` D14 末尾）。
#[test]
fn the_pipeline_link_refusal_carries_the_2_8_2_wording() {
    let workspace = Workspace::new();
    workspace.start();
    workspace.handoff("fresh");
    let output = workspace.engine(&[
        "report",
        "--stage",
        "reverse-engineering",
        "--result",
        "awaiting-approval",
        "--user-input",
        "Approve",
    ]);
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        text(&value, "message"),
        concat!(
            "Cannot present \"reverse-engineering\" for approval because these pipeline handoffs ",
            "have not been recorded for the current run: aidlc-developer-agent, ",
            "aidlc-architect-agent. Re-run `aidlc engine orchestrate next` and dispatch the ",
            "missing pipeline links in their declared order, carrying the human's revision ",
            "feedback. Rejection starts a new attempt: earlier scans and receipts cannot certify ",
            "this revision, even for a targeted artifact edit. After each link returns, run ",
            "`aidlc engine log link --stage reverse-engineering --link <agent>`. Do not ",
            "re-stamp an old handoff or disable evidence checks to reopen the gate.",
        ),
        "{value:?}"
    );
    // 登録 repo の無い intent では `--repo <repo>` を足さない（2.8.2 の条件付き分岐）。
    assert!(
        !text(&value, "message").contains("--repo <repo>"),
        "{value:?}"
    );
}

/// 隔離実行（`--single`）の拒否文言も、綴りと案内文が 2.8.2 と一致する。
///
/// 2.8.2（`.claude/tools/aidlc-orchestrate.ts:7916-7928`）は `singleRun` のとき 3 か所を変える
/// — 冒頭を `Cannot complete an isolated run of "<slug>"` に替え、再実行指示へ
/// ` --single --stage <slug>` を足し、`log link` の末尾へ ` --single` を足す。通常実行の文言
/// （`Cannot present "…" for approval`）が混ざらないことも併せて固定する。
#[test]
fn the_isolated_run_pipeline_link_refusal_carries_the_2_8_2_wording() {
    let workspace = Workspace::new();
    workspace.start();
    workspace.handoff("fresh");
    // 隔離実行の試行を開く（受領証は 1 件も記録しない）。
    let started = workspace.engine(&["next", "--stage", "reverse-engineering", "--single"]);
    assert!(started.status.success(), "{started:?}");
    let output = workspace.engine(&[
        "report",
        "--stage",
        "reverse-engineering",
        "--single",
        "--result",
        "completed",
    ]);
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        text(&value, "message"),
        concat!(
            "Cannot complete an isolated run of \"reverse-engineering\" because these pipeline ",
            "handoffs have not been recorded for this isolated run: aidlc-developer-agent, ",
            "aidlc-architect-agent. Re-run `aidlc engine orchestrate next --single --stage ",
            "reverse-engineering` and dispatch the missing pipeline links in their declared ",
            "order, carrying the human's revision feedback. Rejection starts a new attempt: ",
            "earlier scans and receipts cannot certify this revision, even for a targeted ",
            "artifact edit. After each link returns, run `aidlc engine log link --stage ",
            "reverse-engineering --link <agent> --single`. Do not re-stamp an old handoff or ",
            "disable evidence checks to reopen the gate.",
        ),
        "{value:?}"
    );
    // 通常実行の文言と、到達しない `--repo` 分岐が混ざらない。
    for absent in ["for approval", "--repo <repo>"] {
        assert!(!text(&value, "message").contains(absent), "{value:?}");
    }
}

#[test]
fn single_pipeline_requires_its_own_open_attempt_and_receipts() {
    let workspace = Workspace::new();
    workspace.start();
    workspace.handoff("fresh");
    assert!(
        workspace
            .link("aidlc-developer-agent", false)
            .status
            .success()
    );
    assert!(
        workspace
            .link("aidlc-architect-agent", false)
            .status
            .success()
    );
    let report = [
        "report",
        "--stage",
        "reverse-engineering",
        "--single",
        "--result",
        "completed",
    ];
    let no_start: serde_json::Value =
        serde_json::from_slice(&workspace.engine(&report).stdout).unwrap();
    assert!(
        no_start
            .get("message")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("no open single-stage"),
        "{no_start:?}"
    );
    let next = ["next", "--stage", "reverse-engineering", "--single"];
    let started: serde_json::Value =
        serde_json::from_slice(&workspace.engine(&next).stdout).unwrap();
    assert_eq!(
        started.get("pipeline").unwrap().get("completed").unwrap(),
        &serde_json::Value::Array(vec![])
    );
    let missing: serde_json::Value =
        serde_json::from_slice(&workspace.engine(&report).stdout).unwrap();
    assert!(
        missing
            .get("message")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("have not been recorded for this isolated run"),
        "{missing:?}"
    );
    assert!(
        workspace
            .link("aidlc-developer-agent", true)
            .status
            .success()
    );
    assert!(
        workspace
            .link("aidlc-architect-agent", true)
            .status
            .success()
    );
    let resumed: serde_json::Value =
        serde_json::from_slice(&workspace.engine(&next).stdout).unwrap();
    assert_eq!(
        resumed
            .get("pipeline")
            .unwrap()
            .get("completed")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let completed: serde_json::Value =
        serde_json::from_slice(&workspace.engine(&report).stdout).unwrap();
    assert_eq!(
        completed.get("kind").unwrap().as_str(),
        Some("done"),
        "{completed:?}"
    );
    assert_eq!(workspace.next_completed().as_array().unwrap().len(), 2);
    let second: serde_json::Value =
        serde_json::from_slice(&workspace.engine(&next).stdout).unwrap();
    assert_eq!(
        second
            .get("pipeline")
            .unwrap()
            .get("completed")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn pipeline_progress_and_gate_refusals_match_the_fixed_upstream_observations() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/pipeline-progress-human-revision.json"
    ))
    .unwrap();
    let workspace = Workspace::new();
    workspace.start();
    workspace.handoff("fresh");
    for observed in corpus.get("observations").unwrap().as_array().unwrap() {
        match text(observed, "id") {
            "developer" => assert!(
                workspace
                    .link("aidlc-developer-agent", false)
                    .status
                    .success()
            ),
            "complete" => assert!(
                workspace
                    .link("aidlc-architect-agent", false)
                    .status
                    .success()
            ),
            "expired" => {
                workspace.handoff("changed");
            }
            "human-backed-rejection" => {
                use std::io::Write as _;
                let mut child = Command::new(env!("CARGO_BIN_EXE_aidlc"))
                    .args(["hook", "record-human-turn"])
                    .current_dir(workspace.root())
                    .env_clear()
                    .envs(coverage_profile_env())
                    .env("HOME", workspace.temp.path().join("home"))
                    .env("PATH", "/usr/bin:/bin")
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .unwrap();
                child.stdin.take().unwrap().write_all(br#"{"hook_event_name":"UserPromptSubmit","session_id":"pipeline-test","prompt":"Request Changes: test revision"}"#).unwrap();
                let output = child.wait_with_output().unwrap();
                assert!(output.status.success(), "{output:?}");
            }
            _ => {}
        }
        if text(observed, "kind") == "next" {
            let output = workspace.engine(&["next", "--resume"]);
            let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(
                value.get("pipeline"),
                observed.get("pipeline"),
                "{}: {value:?}",
                text(observed, "id")
            );
        } else {
            let args = observed
                .get("args")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_str().unwrap())
                .collect::<Vec<_>>();
            let output = workspace.engine(&args);
            assert_eq!(
                String::from_utf8(output.stdout).unwrap(),
                text(observed, "stdout"),
                "{} stdout",
                text(observed, "id")
            );
            assert_eq!(
                String::from_utf8(output.stderr).unwrap(),
                text(observed, "stderr"),
                "{} stderr",
                text(observed, "id")
            );
            assert_eq!(
                output.status.code().map(i64::from),
                observed.get("exit").unwrap().as_i64()
            );
        }
    }
}
#[test]
fn an_open_gate_cannot_reuse_receipts_after_the_handoff_changes() {
    let workspace = Workspace::new();
    workspace.start();
    workspace.handoff("fresh");
    assert!(
        workspace
            .link("aidlc-developer-agent", false)
            .status
            .success()
    );
    assert!(
        workspace
            .link("aidlc-architect-agent", false)
            .status
            .success()
    );
    let open = [
        "report",
        "--stage",
        "reverse-engineering",
        "--result",
        "awaiting-approval",
    ];
    let initial: serde_json::Value =
        serde_json::from_slice(&workspace.engine(&open).stdout).unwrap();
    assert_eq!(
        initial.get("kind").and_then(serde_json::Value::as_str),
        Some("print"),
        "{initial:?}"
    );
    workspace.handoff("changed");
    for args in [
        open.to_vec(),
        vec![
            "report",
            "--stage",
            "reverse-engineering",
            "--result",
            "approved",
            "--user-input",
            "Approve",
        ],
    ] {
        let value: serde_json::Value =
            serde_json::from_slice(&workspace.engine(&args).stdout).unwrap();
        assert!(
            value
                .get("message")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("pipeline handoffs have not been recorded"),
            "{value:?}"
        );
    }
}
#[test]
fn nonpipeline_single_reports_also_require_a_start_boundary() {
    let workspace = Workspace::new();
    workspace.start();
    let report = [
        "report",
        "--stage",
        "requirements-analysis",
        "--single",
        "--result",
        "completed",
    ];
    let first: serde_json::Value =
        serde_json::from_slice(&workspace.engine(&report).stdout).unwrap();
    assert!(
        first
            .get("message")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("no open single-stage"),
        "{first:?}"
    );
    let next = ["next", "--stage", "requirements-analysis", "--single"];
    let start = workspace.engine(&next);
    assert!(start.status.success());
    let completed: serde_json::Value =
        serde_json::from_slice(&workspace.engine(&report).stdout).unwrap();
    assert_eq!(
        completed.get("kind").and_then(serde_json::Value::as_str),
        Some("done"),
        "{completed:?}"
    );
    let repeated: serde_json::Value =
        serde_json::from_slice(&workspace.engine(&report).stdout).unwrap();
    assert!(
        repeated
            .get("message")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("no open single-stage"),
        "{repeated:?}"
    );
}

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;

#[path = "../../../../tests/support/tool_link.rs"]
mod tool_link;
