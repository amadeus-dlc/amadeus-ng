//! runtime-graph 再構築フックを、固定本家 2.7.1 の発火条件と同じ公開境界で検証する。
//!
//! 対象は `hooks/aidlc-rebuild-stage-graph.ts:110-259`（発火条件）と
//! `tools/aidlc-runtime.ts:782-808`（compile が書くもの）である。静的な
//! `stage-graph.json` の読取りとは別物であり、compile は runtime-graph を書くだけでなく
//! 承認済みかつ日誌が空の stage へ重複を抑えた `MEMORY_EMPTY` を記録する。
// 契約テストは固定フィクスチャの添字参照と panic を検証の合図として使う。封筒の JSON は
// 本家がフックへ渡すワイヤ形式そのものなので、契約 JSON の直列化規約 (BR1.7) の射程外である。
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::disallowed_methods
)]
use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

/// PostToolUse 封筒（本家は Bash matcher で受ける）。
fn envelope(command: &str) -> String {
    format!(
        r#"{{"session_id":"11111111-2222-4333-8444-555555555555","tool_name":"Bash","tool_input":{{"command":{}}},"tool_response":{{"stdout":""}}}}"#,
        serde_json::to_string(command).unwrap()
    )
}

/// 遷移を書く公開面の 1 つ（`aidlc-orchestrate.ts report`）。
const REPORT_COMMAND: &str = "bun .claude/tools/aidlc-orchestrate.ts report --result completed";

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
        // 本家 `intent-create` は Greenfield と走査したワークスペースで reverse-engineering を
        // SKIP する (`aidlc-utility.ts:5895-5904`)。この fixture は reverse-engineering が最初の
        // run-stage であることに依るので、ソースを 1 つ置いて Brownfield にする。
        fs::create_dir_all(root.path().join("src")).unwrap();
        fs::write(root.path().join("src/lib.rs"), "pub fn smoke() {}\n").unwrap();
        let fixture = Self {
            record: PathBuf::new(),
            root,
        };
        let created = fixture.cli(
            "aidlc-utility",
            &[
                "intent-create",
                "--scope",
                "bugfix",
                "--label",
                "runtime-graph",
                "--arguments",
                "Verify runtime graph rebuild",
            ],
        );
        assert!(created.status.success(), "{created:?}");
        let intents = fixture.root.path().join("aidlc/spaces/default/intents");
        let record = intents.join(
            fs::read_to_string(intents.join("active-intent"))
                .unwrap()
                .trim(),
        );
        Self {
            root: fixture.root,
            record,
        }
    }

    /// 初期化 3 段を畳んで inception へ入る（`state-init` が `[x]` になる）。
    fn advance(&self) {
        let next = self.cli("aidlc-orchestrate", &["next", "bugfix"]);
        assert!(next.status.success(), "{next:?}");
    }

    fn cli(&self, argv0: &str, args: &[&str]) -> Output {
        let binary = self.root.path().join(argv0);
        if !binary.exists() {
            tool_link::link_tool(&binary).unwrap();
        }
        Command::new(binary)
            .args(args)
            .current_dir(self.root.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.root.path())
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap()
    }

    fn rebuild(&self, input: &str) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .args(["hook", "rebuild-stage-graph"])
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

    fn shard(&self) -> PathBuf {
        fs::read_dir(self.record.join("audit"))
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| path.extension().is_some_and(|ext| ext == "md"))
            .unwrap()
    }

    fn audit(&self) -> String {
        fs::read_to_string(self.shard()).unwrap()
    }

    fn memory_empty_rows(&self) -> Vec<String> {
        self.audit()
            .split("\n---\n")
            .filter(|block| block.contains("**Event**: MEMORY_EMPTY\n"))
            .map(str::to_string)
            .collect()
    }

    /// `<record>/<phase>/<stage>/memory.md` を合成入力として置く。
    fn write_journal(&self, phase: &str, stage: &str, body: &str) {
        let dir = self.record.join(phase).join(stage);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("memory.md"), body).unwrap();
    }

    fn graph_path(&self) -> PathBuf {
        self.record.join("runtime-graph.json")
    }

    /// 失敗時の材料 — フック落ちの記録を含む記録ツリー。
    fn tree(&self) -> String {
        fn walk(dir: &Path, out: &mut Vec<String>) {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    if path.is_dir() {
                        walk(&path, out);
                    } else if path.extension().is_some_and(|ext| ext == "drops") {
                        out.push(format!(
                            "{}: {}",
                            path.display(),
                            fs::read_to_string(&path).unwrap_or_default()
                        ));
                    } else {
                        out.push(path.display().to_string());
                    }
                }
            }
        }
        let mut out = Vec::new();
        walk(self.root.path(), &mut out);
        out.join("\n")
    }

    fn graph(&self) -> serde_json::Value {
        let raw = fs::read_to_string(self.graph_path())
            .unwrap_or_else(|error| panic!("{error}\n{}", self.tree()));
        serde_json::from_str(&raw).unwrap()
    }
}

/// オブジェクトのキーを宣言順に読む（`preserve_order` 前提）。
fn keys(value: &serde_json::Value) -> Vec<&str> {
    value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}

/// 記録の無い日誌の骨組み（4 見出しだけ — 記録は 0 件）。
const EMPTY_JOURNAL: &str =
    "# Stage Memory\n\n## Interpretations\n\n## Deviations\n\n## Tradeoffs\n\n## Open questions\n";

/// 記録が 1 件ある日誌。
const FILLED_JOURNAL: &str = "# Stage Memory\n\n## Interpretations\n\n- 2026-09-09T00:00:00Z — 解釈; 文脈\n\n## Deviations\n\n## Tradeoffs\n\n## Open questions\n";

/// 本家は report 後の Bash と監査末尾の対象遷移で compile を発火し、runtime-graph を書く。
#[test]
fn a_transition_reporting_command_compiles_the_runtime_graph() {
    let fixture = Fixture::new();
    fixture.advance();
    let result = fixture.rebuild(&envelope(REPORT_COMMAND));
    assert!(
        result.status.success() && result.stdout.is_empty() && result.stderr.is_empty(),
        "{result:?}"
    );
    let graph = fixture.graph();
    // 封筒のキーの並びは契約である（本家 `RuntimeGraph` / `RuntimeStage` の宣言順）。
    assert_eq!(
        keys(&graph),
        vec!["workflow_id", "scope", "started_at", "stages"]
    );
    assert_eq!(graph["scope"], "bugfix");
    assert_eq!(graph["workflow_id"], graph["started_at"]);
    let stages = graph["stages"].as_array().unwrap();
    let state_init = stages
        .iter()
        .find(|row| row["stage_slug"] == "state-init")
        .unwrap();
    assert_eq!(
        keys(state_init),
        vec![
            "stage_slug",
            "started_at",
            "completed_at",
            "agent",
            "memory_path",
            "memory_entries",
            "memory_breakdown",
            "sensor_firings",
            "outcome",
            "learnings_captured",
        ]
    );
    assert_eq!(state_init["outcome"], "approved");
    assert!(state_init["started_at"].is_string());
    assert!(state_init["completed_at"].is_string());
    assert_eq!(state_init["agent"], "orchestrator");
    assert_eq!(
        state_init["memory_path"],
        serde_json::Value::String(format!(
            "aidlc/spaces/default/intents/{}/initialization/state-init/memory.md",
            fixture.record.file_name().unwrap().to_str().unwrap()
        ))
    );
    // 日誌が無い位置は「0 件」ではなく「観測できない」— 本家 `readMemory` の `null`。
    assert!(state_init["memory_entries"].is_null());
    assert!(state_init["memory_breakdown"].is_null());
    assert_eq!(state_init["sensor_firings"].as_array().unwrap().len(), 0);
    assert_eq!(state_init["learnings_captured"]["from_orchestrator"], 0);
    let running = stages
        .iter()
        .find(|row| row["stage_slug"] == "reverse-engineering")
        .unwrap();
    assert_eq!(running["outcome"], "pending");
    assert!(running["completed_at"].is_null());
    assert!(running["learnings_captured"].is_null());
    // 日誌が無いので `MEMORY_EMPTY` は 1 件も出ない。
    assert!(fixture.memory_empty_rows().is_empty());
}

/// 遷移と無関係なツール入力では発火しない。
#[test]
fn an_unrelated_command_never_compiles() {
    let fixture = Fixture::new();
    fixture.advance();
    let result = fixture.rebuild(&envelope("ls -la aidlc/spaces"));
    assert!(
        result.status.success() && result.stdout.is_empty() && result.stderr.is_empty(),
        "{result:?}"
    );
    assert!(!fixture.graph_path().exists(), "compile が発火した");
}

/// compile 自身の呼出しは再入を起こさない（本家のコマンド水準の再帰ガード）。
#[test]
fn the_runtime_tool_itself_never_retriggers_the_compile() {
    let fixture = Fixture::new();
    fixture.advance();
    for command in [
        "bun .claude/tools/aidlc-runtime.ts compile",
        "bun .claude/tools/aidlc-runtime.ts compile && bun .claude/tools/aidlc-state.ts approve",
        "aidlc runtime compile",
    ] {
        let result = fixture.rebuild(&envelope(command));
        assert!(result.status.success(), "{result:?}");
        assert!(
            !fixture.graph_path().exists(),
            "再帰ガードが効いていない: {command}"
        );
    }
}

/// 監査末尾 3 ブロックに対象遷移が無ければ発火しない。
#[test]
fn an_audit_tail_without_a_transition_never_compiles() {
    let fixture = Fixture::new();
    fixture.advance();
    let mut audit = fixture.audit();
    for _ in 0..3 {
        audit.push_str(
            "\n## Human Turn\n**Timestamp**: 2026-09-09T23:00:00Z\n**Event**: HUMAN_TURN\n\n---\n",
        );
    }
    fs::write(fixture.shard(), audit).unwrap();
    let result = fixture.rebuild(&envelope(REPORT_COMMAND));
    assert!(result.status.success(), "{result:?}");
    assert!(!fixture.graph_path().exists(), "compile が発火した");
}

/// 不正 JSON・非 Bash 入力は無視する（本家は exit 0 で黙る）。
#[test]
fn a_malformed_envelope_is_ignored() {
    let fixture = Fixture::new();
    fixture.advance();
    for input in ["not json", "{\"tool_name\":\"Bash\"", "[]"] {
        let result = fixture.rebuild(input);
        assert!(
            result.status.success() && result.stdout.is_empty() && result.stderr.is_empty(),
            "{result:?}"
        );
        assert!(!fixture.graph_path().exists());
    }
}

/// 発火した compile は心拍を残す（`--doctor` が沈黙故障を見るため）。
#[test]
fn a_fired_compile_writes_the_heartbeat() {
    let fixture = Fixture::new();
    fixture.advance();
    fixture.rebuild(&envelope(REPORT_COMMAND));
    assert!(
        fixture
            .record
            .join(".aidlc-hooks-health/rebuild-stage-graph.last")
            .is_file()
    );
}

/// 承認済みで日誌が空の位置には `MEMORY_EMPTY` を 1 件だけ記録する（再発火は抑える）。
#[test]
fn an_empty_journal_of_an_approved_stage_is_recorded_once() {
    let fixture = Fixture::new();
    fixture.advance();
    fixture.write_journal("initialization", "state-init", EMPTY_JOURNAL);
    fixture.rebuild(&envelope(REPORT_COMMAND));
    let first = fixture.memory_empty_rows();
    assert_eq!(first.len(), 1, "1 件だけ記録する:\n{}", fixture.audit());
    assert!(first[0].contains("**Stage**: state-init\n"));
    assert!(first[0].contains("## Memory Empty\n"));
    fixture.rebuild(&envelope(REPORT_COMMAND));
    assert_eq!(
        fixture.memory_empty_rows().len(),
        1,
        "同じ承認について再記録した:\n{}",
        fixture.audit()
    );
    // 観測は runtime-graph の件数にも出る。
    let graph = fixture.graph();
    let state_init = graph["stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["stage_slug"] == "state-init")
        .unwrap()
        .clone();
    assert_eq!(state_init["memory_entries"], 0);
    assert_eq!(state_init["memory_breakdown"]["interpretations"], 0);
}

/// 記録のある日誌と、進行中の位置の空日誌は `MEMORY_EMPTY` の対象にしない。
#[test]
fn a_filled_journal_and_a_running_stage_are_not_recorded() {
    let fixture = Fixture::new();
    fixture.advance();
    fixture.write_journal("initialization", "state-init", FILLED_JOURNAL);
    fixture.write_journal("inception", "reverse-engineering", EMPTY_JOURNAL);
    fixture.rebuild(&envelope(REPORT_COMMAND));
    assert!(
        fixture.memory_empty_rows().is_empty(),
        "承認済みでない位置や記録のある日誌を数えた:\n{}",
        fixture.audit()
    );
    let graph = fixture.graph();
    let state_init = graph["stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["stage_slug"] == "state-init")
        .unwrap()
        .clone();
    assert_eq!(state_init["memory_entries"], 1);
    assert_eq!(state_init["memory_breakdown"]["interpretations"], 1);
}

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;

#[path = "../../../../tests/support/tool_link.rs"]
mod tool_link;
