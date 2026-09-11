//! 開始基準を公開CLIから保存し、通常RMUと再接続で読み戻す契約。
#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::{
    fs,
    path::{Path, PathBuf},
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
        tool_link::link_tool(&temp.path().join("aidlc-utility")).unwrap();
        fs::create_dir_all(root.join("aidlc/spaces/default/intents")).unwrap();
        Self { temp }
    }
    fn root(&self) -> PathBuf {
        self.temp.path().join("workspace")
    }
    fn create(&self) -> Output {
        Command::new(self.temp.path().join("aidlc-utility"))
            .args([
                "intent-create",
                "--scope",
                "bugfix",
                "--label",
                "baseline",
                "--arguments",
                "Fix source baseline",
            ])
            .current_dir(self.root())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.temp.path())
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap()
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
        fs::read_dir(self.record().join("audit"))
            .unwrap()
            .map(|p| fs::read_to_string(p.unwrap().path()).unwrap())
            .collect()
    }
}
fn hash(text: &str) -> String {
    core_infrastructure::hash::sha256_hex(text.as_bytes())
}
fn snapshot(record: &Path, listing: &str) -> PathBuf {
    record
        .join(".aidlc-source-review/code-generation")
        .join(format!(
            "baseline-{}.tsv",
            hash(listing).chars().take(12).collect::<String>()
        ))
}
#[test]
fn start_publishes_the_listing_and_its_exact_hash() {
    let workspace = Workspace::new();
    fs::write(workspace.root().join("source.rs"), "fn main() {}\n").unwrap();
    let output = workspace.create();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/golden/selfhost-stage1/source-baseline.json"
    ))
    .unwrap();
    let reference = corpus.get("start_observation").unwrap();
    let listing = reference.get("listing").unwrap().as_str().unwrap();
    assert_eq!(
        output.status.code(),
        reference
            .get("exit")
            .unwrap()
            .as_i64()
            .map(|code| i32::try_from(code).unwrap())
    );
    assert_eq!(
        output.stderr,
        reference
            .get("stderr")
            .unwrap()
            .as_str()
            .unwrap()
            .as_bytes()
    );
    assert_eq!(
        fs::read_to_string(snapshot(&workspace.record(), listing)).unwrap(),
        listing
    );
    assert_eq!(
        snapshot(&workspace.record(), listing)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap(),
        reference.get("snapshot").unwrap().as_str().unwrap()
    );
    assert!(workspace.audit().contains(&format!(
        "**Source Baseline**: {}\n",
        reference.get("source_baseline").unwrap().as_str().unwrap()
    )));
}

#[tokio::test]
async fn stored_baseline_survives_reopening_and_source_changes() {
    use core_command_domain::workspace::{SpaceName, StorePath};
    use core_command_use_case::orchestration::IntentRepository as _;
    let workspace = Workspace::new();
    fs::write(workspace.root().join("source.rs"), "original\n").unwrap();
    assert!(workspace.create().status.success());
    let record = workspace.record();
    let cursor = aidlc::execution_cursor::ExecutionCursor::read(&record)
        .unwrap()
        .unwrap();
    let store = StorePath::for_space(&workspace.root().join("aidlc"), &SpaceName::default());
    let repository =
        core_command_interface_adapter::orchestration::IntentRepositoryImpl::open(&store).unwrap();
    let intent = repository.find_by_id(cursor.intent_id()).await.unwrap();
    let listing = intent
        .source_baseline()
        .unwrap()
        .listing()
        .unwrap()
        .to_string();
    drop(repository);
    fs::remove_file(snapshot(&record, &listing)).unwrap();
    fs::write(workspace.root().join("source.rs"), "changed after start\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_aidlc"))
        .arg("next")
        .current_dir(workspace.root())
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", workspace.temp.path())
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        fs::read_to_string(snapshot(&record, &listing)).unwrap(),
        listing
    );
    assert_eq!(workspace.audit().matches("**Source Baseline**:").count(), 1);
}

#[test]
fn failed_capture_is_explicitly_unbindable_only_when_git_checkout_exists() {
    for git in [false, true] {
        let workspace = Workspace::new();
        fs::write(workspace.root().join(".aidlc-source-paths.json"), "invalid").unwrap();
        if git {
            fs::create_dir(workspace.root().join(".git")).unwrap();
        }
        let output = workspace.create();
        assert!(output.status.success(), "{output:?}");
        let expected = if git {
            "unbindable".to_string()
        } else {
            format!("sha256:{}", hash(""))
        };
        assert!(
            workspace
                .audit()
                .contains(&format!("**Source Baseline**: {expected}\n"))
        );
        assert_eq!(snapshot(&workspace.record(), "").exists(), !git);
    }
}

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;

#[path = "../../../../tests/support/tool_link.rs"]
mod tool_link;
