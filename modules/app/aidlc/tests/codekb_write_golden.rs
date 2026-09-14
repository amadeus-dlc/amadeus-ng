//! 群 D (`codekb-snapshot` / `codekb-publish`) の CLI ゴールデン — upstream 2.7.1 との
//! **stdout バイト・終了コード**の一致。
//!
//! ゴールデンは `tests/golden/codekb-write/` の下に、固定ピン `a277af21` の配布ツールを
//! `fixture.json` の scenario 宣言から建てた使い捨てワークスペースに対して実行して採ったもの
//! (`scripts/goldens/capture-codekb-write.ts`)。このテストは**同じ宣言から同じツリーを建て**、
//! native の出力が upstream と 1 バイトも違わないことを確かめる。
//!
//! # 合言葉は焼き込まず、同じ手順で採り直す
//!
//! `codekb-publish` は compare-and-swap の合言葉 (`--expect-store` / `--expect-source`) を要る。
//! `snapshot_paths` を持つケースは、採取側と同じく**先に同じワークスペースで
//! `codekb-snapshot --json` を走らせ**、その値を argv の `{store_generation}` /
//! `{source_fingerprint}` へ差し込む。合言葉を定数で焼き込むと、git の版差でフィクスチャが
//! 黙って別経路 (`CODEKB_STORE_CHANGED`) を採り、「一致した」という誤った証拠が残る。
//!
//! # 実ストアには触れない
//!
//! 群 D は書込動詞なので、全ケースを使い捨ての一時ディレクトリで建てて回す。このテストは
//! リポジトリの `aidlc/spaces/*/codekb/` を読みも書きもしない。
//!
//! # 失敗経路の扱い
//!
//! 拒否は stdout 空・exit 1 の一致だけを見る。stderr のエンベロープ形式 (`{"error":..}`) の
//! 横断整合は別 Bolt の宿題であり、ここでは byte 比較しない (群 A の
//! `workflow_authority_golden.rs`、群 B/C の `codekb_authority_golden.rs` と同じ既知の限界)。
#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn golden_root() -> PathBuf {
    repo_root().join("tests/golden/codekb-write")
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
    snapshot_paths: Option<String>,
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
                snapshot_paths: meta
                    .get("snapshot_paths")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
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
    if let Some(symlinks) = scenario
        .get("symlinks")
        .and_then(serde_json::Value::as_object)
    {
        for (relative, target) in symlinks {
            let path = workspace.join(relative);
            fs::create_dir_all(path.parent().expect("親")).expect("親ディレクトリ");
            #[cfg(unix)]
            std::os::unix::fs::symlink(target.as_str().expect("リンク先"), &path)
                .expect("シンボリックリンク");
        }
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

/// native を 1 回走らせ、stdout の実効バイトを返す。
///
/// `main.rs` は `writeln!` で末尾改行を付すので、実効 stdout は line + "\n" である。
async fn invoke(workspace: &Path, argv: &[String]) -> (Vec<u8>, u8) {
    let mut argv: Vec<String> = argv.to_vec();
    argv.push("--project-dir".to_string());
    argv.push(workspace.to_string_lossy().into_owned());
    let completion = aidlc::runtime::run("aidlc-utility", &argv, workspace).await;
    let stdout: Vec<u8> = completion
        .line()
        .map(|line| format!("{line}\n").into_bytes())
        .unwrap_or_default();
    (stdout, completion.code())
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

    // 合言葉は同じワークスペースの snapshot から採る (採取スクリプトと同じ手順)。
    let argv: Vec<String> = match &golden.snapshot_paths {
        None => golden.argv.clone(),
        Some(paths) => {
            let taken = [
                "codekb-snapshot".to_string(),
                "--paths".to_string(),
                paths.clone(),
                "--json".to_string(),
            ];
            let (stdout, code) = invoke(&workspace, &taken).await;
            assert_eq!(code, 0, "{}: 先行 snapshot が失敗した", golden.id);
            let snapshot: serde_json::Value =
                serde_json::from_slice(&stdout).expect("snapshot は JSON");
            let store = snapshot["store_generation"]
                .as_str()
                .expect("store_generation");
            let source = snapshot["source_fingerprint"]
                .as_str()
                .expect("source_fingerprint");
            golden
                .argv
                .iter()
                .map(|arg| {
                    arg.replace("{store_generation}", store)
                        .replace("{source_fingerprint}", source)
                })
                .collect()
        }
    };

    let (stdout, code) = invoke(&workspace, &argv).await;
    assert_eq!(
        String::from_utf8_lossy(&stdout),
        String::from_utf8_lossy(&golden.stdout),
        "{}: stdout バイトが upstream と違う",
        golden.id
    );
    assert_eq!(
        code, golden.exit_code,
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
async fn codekb_snapshot_matches_upstream() {
    assert_prefix("snapshot/").await;
}

#[tokio::test]
async fn codekb_publish_matches_upstream() {
    assert_prefix("publish/").await;
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
        "宣言したケースと採取したゴールデンがずれている — `bun scripts/goldens/capture-codekb-write.ts` で採り直す"
    );
}
