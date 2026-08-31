//! 合成ルートの縦切り — **プロセスをまたいだ** steering 分割配信（issue #49）。
//!
//! b28 のクエリ側テストは鍵を定数として渡していた。ここが実証するのは、その鍵が
//! **ディスク上で永続し、別プロセスの `continue` が読み直して封緘を検証できる**ことである。
//!
//! # なぜ「別プロセス」を名乗れるのか
//!
//! [`aidlc::runtime::run`] は**引数とワークスペース以外に何も持ち越さない** — 状態は
//! すべて `--project-dir` の下のファイルにある。したがって同じ引数で 2 回呼ぶことは、
//! 2 回起動することと**プロセス内の状態については同値**である。実際に鍵の受け渡しは
//! ファイル 1 本（`.aidlc-steering-token-key`）だけを通っており、それが消えれば連鎖は
//! 必ず切れる（下の `a_lost_key_breaks_the_chain` がそれを固定する）。
#![allow(clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

use core_infrastructure::canon_json::{JsonValue, parse};

/// 出荷テンプレートと同じ節構成の状態ファイル（カーソルは `domain-design`）。
const STATE_FILE: &str = concat!(
    "# AI-DLC State Tracking\n\n",
    "## Project Information\n",
    "- **Scope**: classic\n",
    "- **State Version**: 8\n\n",
    "## Stage Progress\n\n",
    "### INITIALIZATION PHASE\n",
    "- [x] state-init — EXECUTE\n\n",
    "### INCEPTION PHASE\n",
    "- [-] domain-design — EXECUTE\n",
    "- [ ] contract-design — EXECUTE\n\n",
    "## Current Status\n",
    "- **Current Stage**: domain-design\n",
    "- **Status**: Running\n",
    "- **Last Updated**: 2026-08-29T16:36:24Z\n",
);

/// 記録ディレクトリの名前（カーソルが指す先）。
const RECORD: &str = "260831-demo-01a02785";

struct Workspace {
    root: tempfile::TempDir,
}

impl Workspace {
    fn create() -> Workspace {
        let workspace = Workspace {
            root: tempfile::tempdir().expect("一時ディレクトリ"),
        };
        workspace.write_definition();
        workspace.write_memory_layer();
        fs::create_dir_all(workspace.record_dir().join("audit")).expect("record");
        fs::write(workspace.record_dir().join("aidlc-state.md"), STATE_FILE).expect("状態ファイル");
        let intents = workspace.path("aidlc/spaces/default/intents");
        fs::write(intents.join("active-intent"), format!("{RECORD}\n")).expect("カーソル");
        workspace
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.path().join(relative)
    }

    fn project_dir(&self) -> &Path {
        self.root.path()
    }

    fn record_dir(&self) -> PathBuf {
        self.path("aidlc/spaces/default/intents").join(RECORD)
    }

    fn key_file(&self) -> PathBuf {
        self.record_dir().join(".aidlc-steering-token-key")
    }

    /// 定義 3 入力 + scope identity を `.claude/` の下へ書く。
    fn write_definition(&self) {
        let data = self.path(".claude/tools/data");
        let scopes = self.path(".claude/scopes");
        fs::create_dir_all(&data).expect("data");
        fs::create_dir_all(&scopes).expect("scopes");
        fs::write(
            data.join("harness.json"),
            r#"{"name":"claude","harnessDir":".claude","rulesSubdir":"rules"}"#,
        )
        .expect("harness.json");
        let node = |slug: &str, number: &str, name: &str, phase: &str| {
            format!(
                r#"{{"slug":"{slug}","number":"{number}","name":"{name}","phase":"{phase}",
                     "execution":"ALWAYS","mode":"inline","lead_agent":"orchestrator",
                     "scopes":["classic"]}}"#
            )
        };
        fs::write(
            data.join("stage-graph.json"),
            format!(
                "[{},{},{}]",
                node("state-init", "0.1", "State Init", "initialization"),
                node("domain-design", "1.1", "Domain Design", "inception"),
                node("contract-design", "1.2", "Contract Design", "inception"),
            ),
        )
        .expect("stage-graph.json");
        fs::write(
            data.join("scope-grid.json"),
            r#"{"classic":{"stages":{"state-init":"EXECUTE","domain-design":"EXECUTE","contract-design":"EXECUTE"}}}"#,
        )
        .expect("scope-grid.json");
        fs::write(
            scopes.join("aidlc-classic.md"),
            "---\nname: classic\n---\n\n# Classic\n",
        )
        .expect("scope identity");
    }

    /// 2 部に割れる大きさの memory 層（org.md + team.md、いずれも 12KiB）。
    fn write_memory_layer(&self) {
        let memory = self.path("aidlc/spaces/default/memory");
        fs::create_dir_all(memory.join("phases")).expect("memory");
        let big = "x".repeat(12 * 1024);
        fs::write(memory.join("org.md"), format!("# Org\n{big}\n")).expect("org.md");
        fs::write(memory.join("team.md"), format!("# Team\n{big}\n")).expect("team.md");
    }
}

/// 1 回の起動。`argv0` がツール面を選ぶ（マルチコール）。
async fn invoke(workspace: &Workspace, args: &[&str]) -> aidlc::runtime::Completion {
    let args: Vec<String> = args.iter().map(|a| (*a).to_string()).collect();
    aidlc::runtime::run("aidlc-orchestrate", &args, workspace.project_dir()).await
}

/// 出た 1 行を JSON として読み直す（stdout 契約 — 1 行・1 directive）。
fn directive_of(completion: &aidlc::runtime::Completion) -> JsonValue {
    let line = completion
        .line()
        .unwrap_or_else(|| panic!("stdout に directive が要る: {completion:?}"));
    assert!(!line.contains('\n'), "1 行である: {line}");
    assert_eq!(completion.code(), 0, "ビジネス経路は exit 0");
    parse(line).expect("directive は JSON")
}

fn field(value: &JsonValue, key: &str) -> Option<JsonValue> {
    match value {
        JsonValue::Object(members) => members.get(key).cloned(),
        _ => None,
    }
}

fn string_of(value: &JsonValue, key: &str) -> String {
    match field(value, key) {
        Some(JsonValue::String(text)) => text,
        other => panic!("{key} は文字列であるべき: {other:?}"),
    }
}

fn number_of(value: &JsonValue, key: &str) -> u64 {
    match field(value, key) {
        Some(JsonValue::Number(core_infrastructure::canon_json::Number::PosInt(n))) => n,
        other => panic!("{key} は非負整数であるべき: {other:?}"),
    }
}

/// **#49 の受入** — `next` → load-steering → `continue` が起動をまたいで成立する。
///
/// 2 回目の起動は 1 回目のメモリを一切引き継がず、ディスク上の鍵だけを頼りに
/// 1 回目が封緘したトークンを開封する。
#[tokio::test]
async fn the_steering_chain_survives_across_invocations() {
    let workspace = Workspace::create();

    // --- 起動 1: next ---
    let first = invoke(
        &workspace,
        &[
            "next",
            "--project-dir",
            &workspace.project_dir().to_string_lossy(),
        ],
    )
    .await;
    let directive = directive_of(&first);
    assert_eq!(string_of(&directive, "kind"), "load-steering");
    assert_eq!(number_of(&directive, "part"), 1);
    assert!(
        number_of(&directive, "parts") >= 2,
        "12KiB 2 本は 28KiB 上限で複数部に割れる"
    );
    let token = string_of(&directive, "continue_token");
    assert!(!token.is_empty());
    assert!(workspace.key_file().exists(), "next が鍵を鋳造して残す");

    // --- 起動 2: continue（1 回目の記憶は無く、ディスクの鍵だけが橋である）---
    let second = invoke(
        &workspace,
        &[
            "continue",
            &token,
            "--project-dir",
            &workspace.project_dir().to_string_lossy(),
        ],
    )
    .await;
    let directive = directive_of(&second);

    assert_eq!(
        string_of(&directive, "kind"),
        "load-steering",
        "2 部目が届く（封緘が別起動で検証できた）"
    );
    assert_eq!(number_of(&directive, "part"), 2, "続きの部が届く");
}

/// 鍵が橋であることの対偶 — 鍵を消すと連鎖は必ず切れ、fail-closed の逐語が出る（I12）。
///
/// これが無いと、上のテストが「たまたま両方が同じ鍵を作り直した」だけでも通ってしまう。
#[tokio::test]
async fn a_lost_key_breaks_the_chain() {
    let workspace = Workspace::create();
    let project = workspace.project_dir().to_string_lossy().into_owned();

    let first = invoke(&workspace, &["next", "--project-dir", &project]).await;
    let token = string_of(&directive_of(&first), "continue_token");

    fs::remove_file(workspace.key_file()).expect("鍵を失う");

    let second = invoke(&workspace, &["continue", &token, "--project-dir", &project]).await;
    let directive = directive_of(&second);

    assert_eq!(string_of(&directive, "kind"), "error");
    assert_eq!(
        string_of(&directive, "message"),
        "Invalid steering continuation token: this stage's rules cannot be loaded from where they left off. Run a fresh `next` to restart delivery from part 1."
    );
}

/// `continue` は鍵を鋳造しない（鋳造してしまうと fail-closed が成立しない）。
#[tokio::test]
async fn continue_does_not_mint_the_key() {
    let workspace = Workspace::create();
    let project = workspace.project_dir().to_string_lossy().into_owned();

    let completion = invoke(
        &workspace,
        &["continue", "whatever", "--project-dir", &project],
    )
    .await;

    assert_eq!(string_of(&directive_of(&completion), "kind"), "error");
    assert!(
        !workspace.key_file().exists(),
        "読むだけの動詞はマシンローカルな鍵すら作らない"
    );
}

/// 改竄されたトークンは fail-closed（MAC 不一致 — I12）。
#[tokio::test]
async fn a_tampered_token_is_refused() {
    let workspace = Workspace::create();
    let project = workspace.project_dir().to_string_lossy().into_owned();

    let first = invoke(&workspace, &["next", "--project-dir", &project]).await;
    let token = string_of(&directive_of(&first), "continue_token");
    let tampered = format!("{token}x");

    let second = invoke(
        &workspace,
        &["continue", &tampered, "--project-dir", &project],
    )
    .await;

    assert_eq!(string_of(&directive_of(&second), "kind"), "error");
}

/// 繰り返し `next` を叩いても鍵は変わらない（同じ連鎖に留まる）。
#[tokio::test]
async fn repeated_next_calls_reuse_one_key() {
    let workspace = Workspace::create();
    let project = workspace.project_dir().to_string_lossy().into_owned();

    invoke(&workspace, &["next", "--project-dir", &project]).await;
    let first = fs::read_to_string(workspace.key_file()).expect("鍵");
    invoke(&workspace, &["next", "--project-dir", &project]).await;
    let second = fs::read_to_string(workspace.key_file()).expect("鍵");

    assert_eq!(first, second);
}

/// 未知動詞は**自己防衛拒否** — stdout へは何も出さず、stderr と exit 1（2 層の出口）。
#[tokio::test]
async fn an_unknown_verb_is_refused_on_stderr_with_a_nonzero_exit() {
    let workspace = Workspace::create();
    let project = workspace.project_dir().to_string_lossy().into_owned();

    let completion = invoke(&workspace, &["frobnicate", "--project-dir", &project]).await;

    assert_eq!(completion.line(), None, "stdout には何も出さない");
    assert_eq!(
        completion.diagnostic(),
        Some("Unknown subcommand: frobnicate. Valid: next, continue, report, park")
    );
    assert_eq!(completion.code(), 1);
}
