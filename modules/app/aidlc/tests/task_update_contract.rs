//! 通常ClaudeのTaskUpdate同期を固定本家の観測と同じ公開境界で検証する。
#![allow(clippy::unwrap_used)]
use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

struct Fixture {
    root: tempfile::TempDir,
    record: PathBuf,
}
impl Fixture {
    fn new() -> Self {
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
        // 採取元 (`test-evidence/runtime-sync-observations.json`) は reverse-engineering が
        // EXECUTE の記録で観測した。本家 `intent-create` は Greenfield と走査したワークスペース
        // で reverse-engineering を SKIP する (`aidlc-utility.ts:5895-5904`) ので、ソースを
        // 1 つ置いて Brownfield にする。
        fs::create_dir_all(root.path().join("src")).unwrap();
        fs::write(root.path().join("src/lib.rs"), "pub fn smoke() {}\n").unwrap();
        let utility = root.path().join("aidlc-utility");
        tool_link::link_tool(&utility).unwrap();
        let created = Command::new(utility)
            .args([
                "intent-create",
                "--scope",
                "bugfix",
                "--label",
                "task-sync",
                "--arguments",
                "Verify TaskUpdate synchronization",
            ])
            .current_dir(root.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", root.path())
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap();
        assert!(created.status.success(), "{created:?}");
        let intents = root.path().join("aidlc/spaces/default/intents");
        let record = intents.join(
            fs::read_to_string(intents.join("active-intent"))
                .unwrap()
                .trim(),
        );
        Self { root, record }
    }
    fn sync(&self, input: &str) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .args(["hook", "sync-workflow-state"])
            .current_dir(self.root.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.root.path())
            .env("PATH", "/usr/bin:/bin")
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
    fn state(&self) -> String {
        fs::read_to_string(self.record.join("aidlc-state.md")).unwrap()
    }
    fn audit(&self) -> String {
        fs::read_dir(self.record.join("audit"))
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "md"))
            .map(|entry| fs::read_to_string(entry.path()).unwrap())
            .collect()
    }
    fn count(&self) -> i64 {
        rusqlite::Connection::open(
            self.root
                .path()
                .join("aidlc/spaces/default/intents/.aidlc-store.sqlite"),
        )
        .unwrap()
        .query_row("SELECT count(*) FROM journal", [], |row| row.get(0))
        .unwrap()
    }
}

#[test]
fn the_current_stage_sync_is_recorded_without_a_success_audit() {
    let fixture = Fixture::new();
    let audit = fixture.audit();
    let count = fixture.count();
    let result = fixture.sync(r#"{"tool_name":"TaskUpdate","tool_input":{"status":"in_progress","activeForm":"Running [reverse-engineering]"}}"#);
    assert!(
        result.status.success() && result.stdout.is_empty() && result.stderr.is_empty(),
        "{result:?}"
    );
    assert_eq!(fixture.count(), count + 1);
    assert_eq!(fixture.audit(), audit);
    assert!(
        fixture
            .state()
            .contains("- **Current Stage**: reverse-engineering\n")
    );
    assert!(
        fixture
            .record
            .join(".aidlc-hooks-health/sync-workflow-state.last")
            .is_file()
    );
}

/// 本家 2.7.1 は別 stage への同期を受理し、旧 stage の checkbox を戻さず両方を `[-]` にする
/// (`test-evidence/runtime-sync-observations.json` の `taskupdate/different-stage`)。
#[test]
fn a_different_stage_sync_moves_the_cursor_and_keeps_the_previous_stage_in_progress() {
    let fixture = Fixture::new();
    let audit = fixture.audit();
    let count = fixture.count();
    let result = fixture.sync(r#"{"tool_name":"TaskUpdate","tool_input":{"status":"in_progress","activeForm":"Running [requirements-analysis]"}}"#);
    assert!(
        result.status.success() && result.stdout.is_empty() && result.stderr.is_empty(),
        "{result:?}"
    );
    assert_eq!(fixture.count(), count + 1);
    assert_eq!(fixture.audit(), audit, "成功監査を追加しない");
    let state = fixture.state();
    for line in [
        "- **Active Agent**: aidlc-product-agent\n",
        "- **In Progress**: requirements-analysis\n",
        "- [-] reverse-engineering — EXECUTE\n",
        "- [-] requirements-analysis — EXECUTE\n",
        "- **Lifecycle Phase**: INCEPTION\n",
        "- **Current Stage**: requirements-analysis\n",
    ] {
        assert!(state.contains(line), "{line:?} が無い:\n{state}");
    }
}

/// 本家 2.7.1 は実効 SKIP の stage も現在位置として受理する
/// (`taskupdate/out-of-scope-stage`: bugfix の market-research)。
#[test]
fn an_out_of_scope_stage_sync_is_accepted_as_the_current_stage() {
    let fixture = Fixture::new();
    let audit = fixture.audit();
    let count = fixture.count();
    let result = fixture.sync(r#"{"tool_name":"TaskUpdate","tool_input":{"status":"in_progress","activeForm":"Running [market-research]"}}"#);
    assert!(
        result.status.success() && result.stdout.is_empty() && result.stderr.is_empty(),
        "{result:?}"
    );
    assert_eq!(fixture.count(), count + 1);
    assert_eq!(fixture.audit(), audit, "成功監査を追加しない");
    let state = fixture.state();
    for line in [
        "- **Active Agent**: aidlc-product-agent\n",
        "- **In Progress**: market-research\n",
        "- [-] reverse-engineering — EXECUTE\n",
        "- [-] market-research — SKIP\n",
        "- **Lifecycle Phase**: IDEATION\n",
        "- **Current Stage**: market-research\n",
    ] {
        assert!(state.contains(line), "{line:?} が無い:\n{state}");
    }
}

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;

#[path = "../../../../tests/support/tool_link.rs"]
mod tool_link;
