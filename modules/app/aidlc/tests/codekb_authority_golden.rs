//! 群 B/C (`project-description` / `codekb-path` / `codekb-scope-diff`) の CLI ゴールデン —
//! upstream 2.7.1 との **stdout バイト・終了コード**の一致。
//!
//! ゴールデンは `tests/golden/codekb-authority/` の下に、固定ピン `a277af21` の配布ツールを
//! `fixture.json` の scenario 宣言から建てた使い捨てワークスペースに対して実行して採ったもの
//! (`scripts/goldens/capture-codekb-authority.ts`)。このテストは**同じ宣言から同じツリーを
//! 建て**、native の出力が upstream と 1 バイトも違わないことを確かめる。
//!
//! # なぜ静的なフィクスチャ木ではなく宣言なのか
//!
//! 群 A のフィクスチャは `.claude` を写した静的な木だったが、群 C は **git 作業ツリーの内容
//! 指紋**を観測する。git リポジトリはリポジトリの中に静的にコミットできない (入れ子の `.git`)
//! ので、フィクスチャは「何を書き、どこを git 化するか」の宣言として持ち、採取側とここが
//! 同じ手順で建てる。`git write-tree` は内容アドレスで作業ディレクトリの絶対パスに依存しない
//! ため、両者は同じ指紋を得る (実測: 別ワークスペースで同一値)。
//!
//! # 失敗経路の扱い
//!
//! 拒否 (サイドカー欠落・`--mint` の `--paths` 欠落・`--compare` の不在ファイル) は stdout 空・
//! exit 1 の一致だけを見る。stderr のエンベロープ形式 (`{"error":..}`) の横断整合は別 Bolt の
//! 宿題であり、ここでは byte 比較しない (群 A の `workflow_authority_golden.rs` と同じ既知の限界)。
#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn golden_root() -> PathBuf {
    repo_root().join("tests/golden/codekb-authority")
}

fn fixture() -> serde_json::Value {
    let path = golden_root().join("fixture.json");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("フィクスチャが読めない ({}): {error}", path.display()));
    serde_json::from_str(&raw).expect("フィクスチャは JSON")
}

/// 1 ケースの採取記録 (`meta.json`) と期待バイト。
struct Golden {
    id: String,
    scenario: String,
    argv: Vec<String>,
    exit_code: u8,
    stdout: Vec<u8>,
}

/// `cli/` 以下の全ケースを id 順に読む。
fn goldens() -> Vec<Golden> {
    let root = golden_root().join("cli");
    let mut found = Vec::new();
    collect(&root, &root, &mut found);
    found.sort_by(|left, right| left.id.cmp(&right.id));
    assert!(!found.is_empty(), "ゴールデンが 1 件も無い");
    found
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<Golden>) {
    for entry in fs::read_dir(dir).expect("ゴールデンの読取") {
        let path = entry.expect("エントリ").path();
        if path.is_dir() {
            collect(root, &path, out);
        } else if path.file_name().is_some_and(|name| name == "meta.json") {
            let dir = path.parent().expect("ケースのディレクトリ");
            let raw = fs::read_to_string(&path).expect("meta.json");
            let meta: serde_json::Value = serde_json::from_str(&raw).expect("meta は JSON");
            let id = dir
                .strip_prefix(root)
                .expect("ケース id")
                .to_string_lossy()
                .replace('\\', "/");
            out.push(Golden {
                id,
                scenario: meta["scenario"].as_str().expect("scenario").to_string(),
                argv: meta["argv"]
                    .as_array()
                    .expect("argv")
                    .iter()
                    .map(|value| value.as_str().expect("argv 要素").to_string())
                    .collect(),
                exit_code: u8::try_from(meta["exit_code"].as_u64().expect("exit_code"))
                    .expect("u8"),
                stdout: fs::read(dir.join("stdout")).expect("stdout ゴールデン"),
            });
        }
    }
}

/// scenario 宣言から使い捨てワークスペースを建てる (採取スクリプトと同じ手順)。
fn build_workspace(scenario: &serde_json::Value, name: &str) -> (tempfile::TempDir, PathBuf) {
    let parent = tempfile::tempdir().expect("一時ディレクトリ");
    let workspace = parent.path().join(name);
    fs::create_dir_all(&workspace).expect("ワークスペース");
    for (relative, body) in scenario["files"].as_object().expect("files") {
        let path = workspace.join(relative);
        fs::create_dir_all(path.parent().expect("親")).expect("親ディレクトリ");
        fs::write(&path, body.as_str().expect("ファイル本文")).expect("ファイル");
    }
    for repo in scenario["git_repos"].as_array().expect("git_repos") {
        git_init(&workspace.join(repo.as_str().expect("リポジトリ位置")));
    }
    (parent, workspace)
}

/// 決定的な git リポジトリを建てる。
///
/// repo-local に `core.excludesFile=/dev/null` と `core.autocrlf=false` を据えるのは、
/// 開発者の global gitignore や改行変換が `git add -A` の結果を変えると内容指紋が環境依存に
/// なるためである (採取スクリプトも同じ 2 設定を据える)。
fn git_init(dir: &Path) {
    for args in [
        vec!["init", "-q", "--initial-branch=main"],
        vec!["config", "core.excludesFile", "/dev/null"],
        vec!["config", "core.autocrlf", "false"],
        vec!["add", "-A"],
    ] {
        let output = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(&args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .output()
            .expect("git を起動できる");
        assert!(
            output.status.success(),
            "git {args:?} に失敗した ({}): {}",
            dir.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// 1 ケースを native で走らせ、stdout バイトと終了コードをゴールデンと突き合わせる。
async fn assert_case(golden: &Golden, scenarios: &serde_json::Value) {
    let scenario = &scenarios[&golden.scenario];
    assert!(
        !scenario.is_null(),
        "{}: 未知の scenario {}",
        golden.id,
        golden.scenario
    );
    let name = scenario["workspace_name"].as_str().expect("workspace_name");
    let (_parent, workspace) = build_workspace(scenario, name);

    let mut argv: Vec<String> = golden
        .argv
        .iter()
        .map(|arg| arg.replace("{workspace}", &workspace.to_string_lossy()))
        .collect();
    argv.push("--project-dir".to_string());
    argv.push(workspace.to_string_lossy().into_owned());

    let completion = aidlc::runtime::run("aidlc-utility", &argv, &workspace).await;

    // main.rs は `writeln!` で末尾改行を付すので、実効 stdout は line + "\n"。
    let stdout: Vec<u8> = completion
        .line()
        .map(|line| format!("{line}\n").into_bytes())
        .unwrap_or_default();

    assert_eq!(
        String::from_utf8_lossy(&stdout),
        String::from_utf8_lossy(&golden.stdout),
        "{}: stdout バイトが upstream と違う",
        golden.id
    );
    assert_eq!(
        completion.code(),
        golden.exit_code,
        "{}: 終了コードが upstream と違う",
        golden.id
    );
}

/// 与えた接頭辞のケースをすべて走らせる。
async fn assert_prefix(prefix: &str) {
    let fixture = fixture();
    let scenarios = &fixture["scenarios"];
    let mut ran = 0;
    for golden in goldens().iter().filter(|g| g.id.starts_with(prefix)) {
        assert_case(golden, scenarios).await;
        ran += 1;
    }
    assert!(ran > 0, "{prefix}: 対象のゴールデンが無い");
}

#[tokio::test]
async fn project_description_matches_upstream() {
    assert_prefix("project-description/").await;
}

#[tokio::test]
async fn codekb_path_matches_upstream() {
    assert_prefix("codekb-path/").await;
}

#[tokio::test]
async fn codekb_scope_diff_matches_upstream() {
    assert_prefix("scope-diff/").await;
}

/// 採取したケースの集合と `fixture.json` の宣言が食い違っていない — ゴールデンを 1 件
/// 取りこぼしても気付けるようにする。
#[test]
fn every_declared_case_has_a_captured_golden() {
    let fixture = fixture();
    let declared: BTreeMap<String, String> = fixture["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .map(|case| {
            (
                case["id"].as_str().expect("id").to_string(),
                case["scenario"].as_str().expect("scenario").to_string(),
            )
        })
        .collect();
    let captured: BTreeMap<String, String> = goldens()
        .into_iter()
        .map(|golden| (golden.id, golden.scenario))
        .collect();
    assert_eq!(
        declared, captured,
        "宣言したケースと採取したゴールデンがずれている — `bun scripts/goldens/capture-codekb-authority.ts` で採り直す"
    );
}
