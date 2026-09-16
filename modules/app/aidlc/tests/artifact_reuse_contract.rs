//! `aidlc engine state reuse-artifact` の受領証経路を、隔離した一時workspaceで終端まで観測する。
//!
//! 観測するのは 1 本の起動が届く先である — 入口の構文 → 受領証の検証 → 実行集約 →
//! 保存 → 投影 → 監査シャードと応答 JSON。記録専用の受領なので `aidlc-state.md` は動かない
//! (オーナー裁定 D12)。定義グラフに無い段の受領は、監査へ 1 行も残さずに拒否される。
// 契約テストは固定の添字参照を検証の合図として使う (既存の契約テストと同じ許容)。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

/// 受領の対象に使う段 — bugfix スコープ 1 周が踏み、定義グラフに在る。
const STAGE: &str = "reverse-engineering";

/// 受領証イベントが積む監査行の見出し。
const EVENT_LINE: &str = "**Event**: ARTIFACT_REUSED";

struct Workspace {
    temp: tempfile::TempDir,
}

impl Workspace {
    /// 鋳造に足るワークスペース — 実ゴールデンの定義 3 ファイルと bugfix スコープを置く。
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
        let workspace = Self { temp };
        workspace.start();
        workspace
    }

    fn root(&self) -> PathBuf {
        self.temp.path().join("workspace")
    }

    /// bugfix スコープの依頼を 1 つ鋳造する（実行カーソルと共有ストアがここで生まれる）。
    fn start(&self) {
        fs::write(self.root().join("source.rs"), "fn main() {}\n").unwrap();
        let output = Command::new(self.temp.path().join("aidlc-utility"))
            .args([
                "intent-create",
                "--scope",
                "bugfix",
                "--label",
                "reuse",
                "--arguments",
                "Reuse existing artifacts",
            ])
            .current_dir(self.root())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.temp.path().join("home"))
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
    }

    /// `aidlc engine <args...>` を 1 回だけ起動する。
    fn engine(&self, args: &[&str]) -> Output {
        let mut argv = vec!["engine"];
        argv.extend_from_slice(args);
        Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .args(&argv)
            .current_dir(self.root())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.temp.path().join("home"))
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap()
    }

    fn record(&self) -> PathBuf {
        let intents = self.root().join("aidlc/spaces/default/intents");
        let name = fs::read_to_string(intents.join("active-intent")).unwrap();
        intents.join(name.trim())
    }

    /// 監査シャードを名前順に連結した全文。
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

    fn receipts(&self) -> usize {
        self.audit().matches(EVENT_LINE).count()
    }

    fn state(&self) -> Vec<u8> {
        fs::read(self.record().join("aidlc-state.md")).unwrap()
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

/// 定義グラフに在る段の受領は成立し、監査行が 1 件だけ増え、状態ファイルは動かない。
#[test]
fn a_receipt_for_a_defined_stage_appends_exactly_one_audit_row_without_moving_the_state() {
    let workspace = Workspace::new();
    let before = workspace.receipts();
    let state = workspace.state();
    let output = workspace.engine(&[
        "state",
        "reuse-artifact",
        STAGE,
        "--decision",
        "keep",
        "--artifacts",
        "docs/a.md",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "受理されなかった — {}",
        stderr(&output).trim()
    );
    let emitted: serde_json::Value = serde_json::from_str(stdout(&output).trim()).unwrap();
    assert_eq!(emitted["emitted"], "ARTIFACT_REUSED");
    assert_eq!(emitted["slug"], STAGE);
    assert_eq!(emitted["decision"], "keep");
    assert_eq!(emitted["artifacts"], "docs/a.md");
    assert!(
        emitted.get("repo").is_none() && emitted.get("single").is_none(),
        "指定の無い任意欄が載った — {emitted}"
    );
    assert_eq!(workspace.receipts(), before + 1, "{}", workspace.audit());
    let audit = workspace.audit();
    assert!(audit.contains(&format!("**Stage**: {STAGE}")), "{audit}");
    assert!(audit.contains("**Decision**: keep"), "{audit}");
    assert!(audit.contains("**Artifacts**: docs/a.md"), "{audit}");
    assert!(
        !audit.contains("**Repo**: ") && !audit.contains("**Workflow**: single-stage:"),
        "指定の無い任意欄が監査へ載った — {audit}"
    );
    assert_eq!(
        workspace.state(),
        state,
        "記録専用の受領が状態ファイルを動かした"
    );
}

/// 定義グラフに無い段の受領は逐語で拒否され、監査行は 1 件も増えない。
#[test]
fn a_receipt_for_a_stage_the_definition_does_not_have_is_refused_without_an_audit_row() {
    let workspace = Workspace::new();
    let before = workspace.receipts();
    let state = workspace.state();
    let misspelled = "reverse-engineerng";
    let output = workspace.engine(&[
        "state",
        "reuse-artifact",
        misspelled,
        "--decision",
        "keep",
        "--artifacts",
        "docs/a.md",
    ]);
    assert_ne!(output.status.code(), Some(0), "拒否が成功で終わった");
    assert!(
        stdout(&output).trim().is_empty(),
        "拒否が stdout へ出力した — {}",
        stdout(&output).trim()
    );
    assert!(
        stderr(&output).contains(&format!("Unknown stage: {misspelled}")),
        "段の不在として名指していない — {}",
        stderr(&output).trim()
    );
    assert_eq!(workspace.receipts(), before, "{}", workspace.audit());
    assert_eq!(workspace.state(), state);
}

/// `--repo` と `--single` は、指定したときだけ応答 JSON と監査行に載る。
#[test]
fn the_optional_repo_and_single_flags_ride_on_both_the_response_and_the_audit_row() {
    let workspace = Workspace::new();
    let before = workspace.receipts();
    let output = workspace.engine(&[
        "state",
        "reuse-artifact",
        STAGE,
        "--decision",
        "modify",
        "--artifacts",
        "docs/a.md,docs/b.md",
        "--repo",
        "app",
        "--single",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "受理されなかった — {}",
        stderr(&output).trim()
    );
    let emitted: serde_json::Value = serde_json::from_str(stdout(&output).trim()).unwrap();
    assert_eq!(emitted["repo"], "app");
    assert_eq!(emitted["single"], true);
    assert_eq!(emitted["decision"], "modify");
    assert_eq!(emitted["artifacts"], "docs/a.md,docs/b.md");
    assert_eq!(workspace.receipts(), before + 1, "{}", workspace.audit());
    let audit = workspace.audit();
    assert!(audit.contains("**Repo**: app"), "{audit}");
    assert!(
        audit.contains(&format!("**Workflow**: single-stage:{STAGE}")),
        "{audit}"
    );
}

/// 閉集合の外の決定は本家の逐語で拒否され、監査行は増えない。
#[test]
fn a_decision_outside_the_closed_set_is_refused_in_the_upstream_wording() {
    let workspace = Workspace::new();
    let before = workspace.receipts();
    let output = workspace.engine(&[
        "state",
        "reuse-artifact",
        STAGE,
        "--decision",
        "bogus",
        "--artifacts",
        "docs/a.md",
    ]);
    assert_ne!(output.status.code(), Some(0), "拒否が成功で終わった");
    assert!(
        stderr(&output).contains("Invalid decision: bogus. Must be keep, modify, or redo."),
        "決定の拒否として名指していない — {}",
        stderr(&output).trim()
    );
    assert_eq!(workspace.receipts(), before, "{}", workspace.audit());
}

/// 値を要るフラグの次がまたフラグなら、受領へ進まずに構文で止まる。
///
/// `--single` だけが値を取らないフラグである。取り違えを黙って通すと、`--decision` の値が
/// 次のフラグ名に化ける。
#[test]
fn a_flag_that_needs_a_value_stops_the_launch_when_the_next_token_is_another_flag() {
    let workspace = Workspace::new();
    let before = workspace.receipts();
    let output = workspace.engine(&[
        "state",
        "reuse-artifact",
        STAGE,
        "--decision",
        "--artifacts",
        "docs/a.md",
    ]);
    assert_ne!(output.status.code(), Some(0), "拒否が成功で終わった");
    assert!(
        stderr(&output).contains(
            "--decision expects a value, got another flag: \"--artifacts\". Did you forget the value?"
        ),
        "構文の拒否として名指していない — {}",
        stderr(&output).trim()
    );
    assert_eq!(workspace.receipts(), before, "{}", workspace.audit());
    // `--single` は値を取らないので、直後のフラグを飲まない。
    let accepted = workspace.engine(&[
        "state",
        "reuse-artifact",
        STAGE,
        "--single",
        "--decision",
        "redo",
        "--artifacts",
        "docs/a.md",
    ]);
    assert_eq!(
        accepted.status.code(),
        Some(0),
        "値を取らないフラグが次のフラグを飲んだ — {}",
        stderr(&accepted).trim()
    );
    assert_eq!(workspace.receipts(), before + 1, "{}", workspace.audit());
}

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;

#[path = "../../../../tests/support/tool_link.rs"]
mod tool_link;
