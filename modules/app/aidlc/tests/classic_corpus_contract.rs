//! classic scope の固定本家 2.7.1 採取列の先頭 (`next --scope classic` → `intent-create` →
//! `next` → `continue <token>` → `next --stage contract-design`) を、配布シェルだけの作業ツリーで
//! native に流し、公開面 (stdout / 状態ファイル / 監査シャード) を
//! `tests/golden/upstream-a277af21/cli/` の実バイトと突き合わせる。
//!
//! # 前提の再現
//!
//! 採取時の作業ツリー (`scripts/goldens/capture-cli.ts` の `makeWorkspace`) は配布シェルの
//! `.claude` と `aidlc` だけを置く。ここでは配布シェルの部分集合を
//! `tests/golden/selfhost-stage1/classic-shell/` (固定コミット a277af21 の `dist/claude` から
//! 写した memory 層・conductor persona・classic の scope 定義・active-space) と
//! `tests/golden/upstream-a277af21/data/` (配布グラフ 3 入力) から組み、どのファイルも封印済みの
//! `source-manifest.sha256` と一致することを先に確かめる。環境変数は `cli/provenance.json` の
//! `non_interactive_env` を含め、採取時と同じ集合にする。
//!
//! # 正規化
//!
//! `tests/golden/upstream-a277af21/normalization.json` の規則をそのまま当てる —
//! 作業ツリーの絶対パス (realpath 解決前後) → `<ROOT>`、監査シャード名とホスト名 → `<CLONE>`、
//! ISO 8601 UTC → `<TS>`、記録ディレクトリ名の日付 (`intents/<YYMMDD>-` / `<YYMMDD>-golden`) →
//! `<TS>`。固定文言・識別子はそれ以外に潰さない。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// 固定本家 2.7.1 のコミット (`cli/provenance.json` の `upstream_commit`)。
const UPSTREAM_COMMIT: &str = "a277af218f0df7f325d3b8be7b6d90fce2c5bd40";

/// 採取時の intent ラベル (`cli/provenance.json` の `fixture_intent_label`)。
const FIXTURE_LABEL: &str = "golden";

/// 採取時の依頼文 (`cli/intent-create/classic-scope/argv` の `--arguments`)。
const FIXTURE_ARGUMENTS: &str = "Build a small ordering service";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn corpus_root() -> PathBuf {
    repo_root().join("tests/golden/upstream-a277af21")
}

fn shell_fixture_root() -> PathBuf {
    repo_root().join("tests/golden/selfhost-stage1/classic-shell")
}

/// 採取済みケースの 1 ファイルを読む。
fn recorded(case: &str, file: &str) -> String {
    let path = corpus_root().join("cli").join(case).join(file);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("ゴールデン {} が読めない: {error}", path.display()))
}

/// 採取済みケースの stdout (末尾改行を含む実バイト)。
fn recorded_stdout(case: &str, file: &str) -> String {
    recorded(case, file)
}

/// 封印済みの配布物マニフェスト (`<sha256>  <relative path>` の 277 行)。
fn sealed_manifest() -> BTreeMap<String, String> {
    let text = fs::read_to_string(corpus_root().join("source-manifest.sha256")).unwrap();
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let (hex, rel) = line
                .split_once("  ")
                .unwrap_or_else(|| panic!("マニフェスト行の形が違う: {line}"));
            (rel.to_string(), hex.to_string())
        })
        .collect()
}

/// 配布シェルの 1 ファイルを、封印済みマニフェストのダイジェストと照合してから写す。
fn copy_verified(
    manifest: &BTreeMap<String, String>,
    source: &Path,
    relative: &str,
    destination_root: &Path,
) {
    let bytes = fs::read(source)
        .unwrap_or_else(|error| panic!("フィクスチャ {} が読めない: {error}", source.display()));
    let expected = manifest
        .get(relative)
        .unwrap_or_else(|| panic!("封印済みマニフェストに {relative} が無い"));
    let actual = core_infrastructure::hash::sha256_hex(&bytes);
    assert_eq!(
        &actual, expected,
        "{relative} は固定本家 {UPSTREAM_COMMIT} の配布実バイトではない"
    );
    let destination = destination_root.join(relative);
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::write(&destination, bytes).unwrap();
}

fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            walk(&path, found);
        } else {
            found.push(path);
        }
    }
}

/// 採取時と同じ非対話環境 (`cli/provenance.json` の `non_interactive_env`)。
fn non_interactive_env() -> Vec<(String, String)> {
    let provenance: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(corpus_root().join("cli/provenance.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        provenance["upstream_commit"].as_str().unwrap(),
        UPSTREAM_COMMIT,
        "コーパスの採取元が固定コミットと違う"
    );
    assert_eq!(
        provenance["fixture_intent_label"].as_str().unwrap(),
        FIXTURE_LABEL
    );
    provenance["non_interactive_env"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(key, value)| (key.clone(), value.as_str().unwrap().to_string()))
        .collect()
}

/// 配布シェルだけを置いた作業ツリーと、その外に置いたバイナリ・HOME。
struct Workspace {
    root: tempfile::TempDir,
    environment: Vec<(String, String)>,
}

impl Workspace {
    /// 固定本家の配布シェル (部分集合) だけを置いた fresh な作業ツリー。
    fn classic() -> Workspace {
        let root = tempfile::tempdir().unwrap();
        let workspace = root.path().join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        let manifest = sealed_manifest();
        let fixture = shell_fixture_root();
        let mut files = Vec::new();
        walk(&fixture, &mut files);
        files.sort();
        for path in &files {
            let relative = path.strip_prefix(&fixture).unwrap().to_str().unwrap();
            if relative == "provenance.json" {
                continue;
            }
            copy_verified(&manifest, path, relative, &workspace);
        }
        for name in ["harness.json", "scope-grid.json", "stage-graph.json"] {
            copy_verified(
                &manifest,
                &corpus_root().join("data").join(name),
                &format!(".claude/tools/data/{name}"),
                &workspace,
            );
        }
        // バイナリと HOME は作業ツリーの外に置く (走査対象に余分なファイルを混ぜない)。
        let bin = root.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        for tool in ["aidlc-orchestrate", "aidlc-utility", "aidlc-jump"] {
            tool_link::link_tool(&bin.join(tool)).unwrap();
        }
        fs::create_dir_all(root.path().join("home")).unwrap();
        Workspace {
            root,
            environment: non_interactive_env(),
        }
    }

    fn path(&self) -> PathBuf {
        self.root.path().join("workspace")
    }

    /// 採取時の argv どおりに `--project-dir` を末尾へ付けて 1 コマンドを流す。
    fn run(&self, tool: &str, args: &[&str]) -> Output {
        let project = self.path();
        let mut command = Command::new(self.root.path().join("bin").join(tool));
        command
            .args(args)
            .arg("--project-dir")
            .arg(&project)
            .current_dir(&project)
            .env_clear()
            .envs(coverage_profile_env())
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", self.root.path().join("home"))
            .env("LANG", "C.UTF-8")
            .env("LC_ALL", "C.UTF-8")
            .env("TZ", "UTC")
            .env("CLAUDE_PROJECT_DIR", &project)
            .env("AIDLC_PROJECT_DIR", &project)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in &self.environment {
            command.env(key, value);
        }
        command.output().unwrap()
    }

    /// 採取時と同じ引数で intent を鋳造する (`cli/intent-create/classic-scope/argv`)。
    fn create_intent(&self) -> Output {
        self.run(
            "aidlc-utility",
            &[
                "intent-create",
                "--scope",
                "classic",
                "--label",
                FIXTURE_LABEL,
                "--arguments",
                FIXTURE_ARGUMENTS,
            ],
        )
    }

    fn intents_dir(&self) -> PathBuf {
        self.path().join("aidlc/spaces/default/intents")
    }

    fn record_dir(&self) -> PathBuf {
        let cursor = fs::read_to_string(self.intents_dir().join("active-intent")).unwrap();
        self.intents_dir().join(cursor.trim())
    }

    fn state(&self) -> String {
        fs::read_to_string(self.record_dir().join("aidlc-state.md")).unwrap()
    }

    /// 監査シャード (採取時と同じく 1 クローンなので 1 ファイル) の全文と、その basename。
    fn audit(&self) -> (String, String) {
        let mut shards: Vec<PathBuf> = fs::read_dir(self.record_dir().join("audit"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
            .collect();
        shards.sort();
        assert_eq!(shards.len(), 1, "監査シャードは 1 クローンぶん: {shards:?}");
        let shard = shards.remove(0);
        let clone = shard.file_stem().unwrap().to_str().unwrap().to_string();
        (fs::read_to_string(shard).unwrap(), clone)
    }

    /// `normalization.json` の規則を当てる (実行時値だけを潰す)。
    fn normalize(&self, text: &str, clone: Option<&str>) -> String {
        let mut out = text.to_string();
        let workspace = self.path();
        let mut roots = vec![workspace.to_string_lossy().into_owned()];
        if let Ok(real) = fs::canonicalize(&workspace) {
            let real = real.to_string_lossy().into_owned();
            if !roots.contains(&real) {
                roots.push(real);
            }
        }
        roots.sort_by_key(|root| std::cmp::Reverse(root.len()));
        for root in roots {
            out = out.replace(&root, "<ROOT>");
        }
        if let Some(clone) = clone {
            out = out.replace(clone, "<CLONE>");
        }
        if let Ok(host) = hostname::get() {
            let host = host.to_string_lossy().into_owned();
            if !host.is_empty() {
                out = out.replace(&host, "<CLONE>");
            }
        }
        let timestamp =
            regex::Regex::new(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z").unwrap();
        out = timestamp.replace_all(&out, "<TS>").into_owned();
        let record = regex::Regex::new(r"intents/\d{6}-").unwrap();
        out = record.replace_all(&out, "intents/<TS>-").into_owned();
        let bare = regex::Regex::new(&format!(r"\d{{6}}-{FIXTURE_LABEL}")).unwrap();
        out = bare
            .replace_all(&out, format!("<TS>-{FIXTURE_LABEL}").as_str())
            .into_owned();
        out
    }
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

/// 採取済みケースの `exit` / `stderr` と突き合わせる。
fn assert_exit_and_stderr(case: &str, output: &Output) {
    let expected_exit: i32 = recorded(case, "exit").trim().parse().unwrap();
    assert_eq!(
        output.status.code(),
        Some(expected_exit),
        "{case}: {output:?}"
    );
    assert_eq!(
        stderr_of(output),
        recorded(case, "stderr"),
        "{case}: stderr"
    );
}

// ---------------------------------------------------------------------------
// cli/next/no-active-intent
// ---------------------------------------------------------------------------

/// 作業が 1 件も無い (`intents/` 不在の) ワークスペースでの `next --scope classic` は、本家どおり
/// 開始コマンドを名指す `print` を返す (`aidlc-orchestrate.ts:1628-1684` @a277af21 —
/// `createPrintDirective`。件数は `costClause` の greenfield 調整後の値)。
#[test]
fn a_fresh_classic_workspace_names_the_recorded_creation_command() {
    let workspace = Workspace::classic();
    assert!(
        !workspace.intents_dir().exists(),
        "採取時の作業ツリーに intents/ は無い"
    );

    let output = workspace.run("aidlc-orchestrate", &["next", "--scope", "classic"]);

    assert_exit_and_stderr("next/no-active-intent", &output);
    assert_eq!(
        stdout_of(&output),
        recorded_stdout("next/no-active-intent", "stdout.json"),
        "採取済みの stdout と 1 バイトも違わない"
    );
}

// ---------------------------------------------------------------------------
// cli/intent-create/classic-scope
// ---------------------------------------------------------------------------

/// greenfield と判定したワークスペースでの classic の `intent-create` は、本家どおり
/// `2.1 (reverse-engineering — greenfield)` を SKIP に畳み、25 stages・`practices-discovery` 着地・
/// Active Agent `aidlc-pipeline-deploy-agent` の状態ファイルを起こす
/// (`aidlc-utility.ts:5784-5793` @a277af21 — greenfield 調整)。stdout も採取どおりである。
#[test]
fn creating_a_classic_intent_on_a_greenfield_workspace_matches_the_recorded_state_file() {
    let workspace = Workspace::classic();

    let output = workspace.create_intent();

    assert_exit_and_stderr("intent-create/classic-scope", &output);
    assert_eq!(
        workspace.normalize(&stdout_of(&output), None),
        recorded("intent-create/classic-scope", "stdout.txt"),
        "採取済みの stdout"
    );
    assert_eq!(
        workspace.normalize(&workspace.state(), None),
        recorded("intent-create/classic-scope", "state-full.md"),
        "誕生直後の状態ファイル全文"
    );
}

/// 同じ鋳造が起こす監査シャードは、`Request` 欄を `/aidlc <arguments>` で書く本家の 16 行と
/// 一致する (`aidlc-utility.ts:5550,5625,5683,5971` @a277af21 —
/// `` `/aidlc ${flags.arguments || scope}` ``)。
#[test]
fn creating_a_classic_intent_matches_the_recorded_audit_shard() {
    let workspace = Workspace::classic();

    let output = workspace.create_intent();

    assert_exit_and_stderr("intent-create/classic-scope", &output);
    let (audit, clone) = workspace.audit();
    assert_eq!(
        workspace.normalize(&audit, Some(&clone)),
        recorded("intent-create/classic-scope", "audit.md"),
        "誕生直後の監査シャード全文"
    );
}

// ---------------------------------------------------------------------------
// cli/next/start → cli/continue/load-steering → cli/next/stage-jump-print
// ---------------------------------------------------------------------------

/// `continue_token` は実行ごとに変わる (`aidlc-orchestrate.ts:3640-3660` の MAC 封緘) ので、
/// 最後のキーであるそれを外した本文を比べる。外し方は綴りの切り出しであって JSON の再構成では
/// ない — キー順・エスケープまで採取どおりであることを保つ。
fn without_continue_token(line: &str) -> (&str, &str) {
    let marker = ",\"continue_token\":\"";
    let at = line
        .rfind(marker)
        .unwrap_or_else(|| panic!("load-steering は continue_token で終わる: {line}"));
    let token = line[at + marker.len()..]
        .trim_end_matches('\n')
        .trim_end_matches('}')
        .trim_end_matches('"');
    (&line[..at], token)
}

/// intent 作成後の最初の `next` は、最初の実行ステージ (`practices-discovery`) の規則束を
/// `load-steering` で配る。`bundle` は本家どおり `sha256:` 接頭辞付きで、素材は規則ファイル
/// 1 つずつの `{path, text}` の配列である (`aidlc-orchestrate.ts:3686` @a277af21 —
/// `` `sha256:${sha256(JSON.stringify(loaded.content))}` ``)。状態ファイルも監査も動かない
/// (採取の `state.diff` / `audit.md` は空)。
#[test]
fn the_first_next_matches_the_recorded_load_steering_except_the_token() {
    let workspace = Workspace::classic();
    assert!(workspace.create_intent().status.success());
    let state_before = workspace.state();
    let (audit_before, _) = workspace.audit();

    let output = workspace.run("aidlc-orchestrate", &["next"]);

    assert_exit_and_stderr("next/start", &output);
    let emitted_line = stdout_of(&output);
    let (emitted, token) = without_continue_token(&emitted_line);
    let recorded_line = recorded("next/start", "stdout.json");
    let (expected, recorded_token) = without_continue_token(&recorded_line);
    // 先に鍵ごとの差を名指す (全文の差分は 17 KB あって読めない)。
    let emitted_json: serde_json::Value = serde_json::from_str(&emitted_line).unwrap();
    let expected_json: serde_json::Value = serde_json::from_str(&recorded_line).unwrap();
    for key in ["kind", "stage", "bundle", "part", "parts"] {
        assert_eq!(
            emitted_json[key], expected_json[key],
            "load-steering の `{key}`"
        );
    }
    assert_eq!(
        emitted_json["rules_content"], expected_json["rules_content"],
        "配る規則の中身と分割"
    );
    assert_eq!(emitted, expected, "continue_token 以外の全文");
    assert!(!token.is_empty() && !recorded_token.is_empty());
    assert_eq!(recorded("next/start", "state.diff"), "");
    assert_eq!(recorded("next/start", "audit.md"), "");
    assert_eq!(
        workspace.state(),
        state_before,
        "next は状態ファイルを動かさない"
    );
    assert_eq!(workspace.audit().0, audit_before, "next は監査を動かさない");
}

/// 配り終えた継続トークンを渡すと、台帳付きの `run-stage` が返る。記録ディレクトリ名の日付
/// だけを潰した全文が採取と一致し、状態ファイルも監査も動かない。
#[test]
fn continuing_the_delivered_token_matches_the_recorded_run_stage() {
    let workspace = Workspace::classic();
    assert!(workspace.create_intent().status.success());
    let first = workspace.run("aidlc-orchestrate", &["next"]);
    assert!(first.status.success(), "{first:?}");
    let first_line = stdout_of(&first);
    let (_, token) = without_continue_token(&first_line);
    let token = token.to_string();
    let state_before = workspace.state();
    let (audit_before, _) = workspace.audit();

    let output = workspace.run("aidlc-orchestrate", &["continue", &token]);

    assert_exit_and_stderr("continue/load-steering", &output);
    assert_eq!(
        workspace.normalize(&stdout_of(&output), None),
        recorded("continue/load-steering", "stdout.json"),
        "run-stage の全文"
    );
    assert_eq!(recorded("continue/load-steering", "state.diff"), "");
    assert_eq!(recorded("continue/load-steering", "audit.md"), "");
    assert_eq!(
        workspace.state(),
        state_before,
        "continue は状態ファイルを動かさない"
    );
    assert_eq!(
        workspace.audit().0,
        audit_before,
        "continue は監査を動かさない"
    );
}

/// `next --stage` は自分で跳ばず、方向と scope を解決した `aidlc-jump.ts execute` を名指す
/// `print` を返す (`aidlc-orchestrate.ts:6593-6640` @a277af21 — `emitJumpDirective`)。
/// 状態ファイルも監査も動かない。
#[test]
fn next_with_a_stage_names_the_recorded_jump_execute_command() {
    let workspace = Workspace::classic();
    assert!(workspace.create_intent().status.success());
    let state_before = workspace.state();
    let (audit_before, _) = workspace.audit();

    let output = workspace.run("aidlc-orchestrate", &["next", "--stage", "contract-design"]);

    assert_exit_and_stderr("next/stage-jump-print", &output);
    assert_eq!(
        stdout_of(&output),
        recorded("next/stage-jump-print", "stdout.json"),
        "採取済みの stdout と 1 バイトも違わない"
    );
    assert_eq!(recorded("next/stage-jump-print", "state.diff"), "");
    assert_eq!(recorded("next/stage-jump-print", "audit.md"), "");
    assert_eq!(workspace.state(), state_before);
    assert_eq!(workspace.audit().0, audit_before);
}

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;

#[path = "../../../../tests/support/tool_link.rs"]
mod tool_link;
