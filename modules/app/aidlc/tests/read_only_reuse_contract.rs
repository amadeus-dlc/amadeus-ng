//! C6 の読取り専用の再利用 — `aidlc-review-brief` の 3 動詞と `aidlc-testing-posture` の
//! 2 動詞が、依存先まで含めて正本 (状態ファイル・監査シャード・イベントストア・承認受領) を
//! 変えないことの実測。
//!
//! ファイル名だけで読取り専用と扱わない (C6)。同じファイルに表示と更新の両方があるので、
//! **操作単位**で前後を突き合わせる。C6 が読取り専用から除外する操作 (受領作成・
//! 承認開始・学習永続化・目録更新) が混ざっていないことも確かめる。
//!
//! 一時ワークスペースへ配布資産を写して検証し、本リポジトリの実ファイルは読むだけである。
// 契約テストは固定の添字参照と panic を検証の合図として使う (既存の契約テストと同じ許容)。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// 記録の中で回す対象ステージ (bugfix の最初の post-initialization ステージ)。
const STAGE: &str = "reverse-engineering";

/// C6 が読取り専用の候補に挙げる 2 本 5 動詞。
const REUSED: [(&str, &str); 5] = [
    ("aidlc-review-brief", "summary"),
    ("aidlc-review-brief", "review"),
    ("aidlc-review-brief", "context"),
    ("aidlc-testing-posture", "render"),
    ("aidlc-testing-posture", "fingerprint"),
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

/// 実際に配布 TypeScript を回せる一時ワークスペース。
struct Workspace {
    temp: tempfile::TempDir,
}

impl Workspace {
    /// 配布資産と memory 層を写し、この build で作業記録を 1 本鋳造したワークスペース。
    async fn minted() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        fs::create_dir_all(root.join(".claude")).unwrap();
        fs::create_dir_all(root.join("aidlc/spaces/default/intents")).unwrap();
        for relative in [
            ".claude/tools",
            ".claude/scopes",
            ".claude/agents",
            ".claude/aidlc-common",
            "aidlc/spaces/default/memory",
        ] {
            copy_tree(&repo_root().join(relative), &root.join(relative));
        }
        fs::copy(
            repo_root().join(".claude/settings.json"),
            root.join(".claude/settings.json"),
        )
        .unwrap();
        let workspace = Self { temp };
        let minted = workspace
            .engine(
                "aidlc-utility",
                &["intent-create", "--scope", "bugfix", "--label", "reuse"],
            )
            .await;
        assert_eq!(minted, 0, "作業記録の鋳造は通る");
        // 最初の directive まで進めて、状態・監査・ストアが揃った姿にする。
        workspace.engine("aidlc-orchestrate", &["next"]).await;
        workspace
    }

    fn root(&self) -> PathBuf {
        self.temp.path().join("workspace")
    }

    /// この build を**プロセス内**で 1 回回す (記録を用意するためだけの足場)。
    ///
    /// これは接続の検証のための手当であって、実地スモーク (FR7) の代わりではない。
    async fn engine(&self, argv0: &str, args: &[&str]) -> u8 {
        let mut owned: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
        owned.push("--project-dir".to_string());
        owned.push(self.root().to_string_lossy().into_owned());
        aidlc::runtime::run(argv0, &owned, &self.root())
            .await
            .code()
    }

    /// 配布 TypeScript を 1 回回す (作業ディレクトリも `AIDLC_PROJECT_DIR` も一時ワークスペース)。
    fn tool(&self, face: &str, args: &[&str]) -> Output {
        let script = self.root().join(".claude/tools").join(format!("{face}.ts"));
        Command::new("bun")
            .arg(script)
            .args(args)
            .current_dir(self.root())
            .env("AIDLC_PROJECT_DIR", self.root())
            .env("CLAUDE_PROJECT_DIR", self.root())
            .env("LANG", "C.UTF-8")
            .env("LC_ALL", "C.UTF-8")
            .env("TZ", "UTC")
            .output()
            .unwrap_or_else(|error| panic!("bun を起動できない: {error}"))
    }

    /// 作業記録の面 (`aidlc/`) の全ファイルの中身 (パス → バイト列)。
    fn canon(&self) -> BTreeMap<String, Vec<u8>> {
        let mut found = BTreeMap::new();
        read_tree(&self.root().join("aidlc"), &self.root(), &mut found);
        found
    }
}

/// 1 動詞ぶんの引数 (動詞ごとの必須フラグを添える)。
fn arguments(verb: &str, questions: &Path) -> Vec<String> {
    let mut args = vec![verb.to_string(), "--stage".to_string(), STAGE.to_string()];
    match verb {
        "summary" => {
            args.push("--questions-file".to_string());
            args.push(questions.to_string_lossy().into_owned());
        }
        "review" => {
            args.push("--why".to_string());
            args.push("first".to_string());
        }
        "fingerprint" => args.push("--stage-level".to_string()),
        _ => {}
    }
    args
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            copy_tree(&entry.path(), &target);
        } else if kind.is_file() {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

fn read_tree(dir: &Path, base: &Path, found: &mut BTreeMap<String, Vec<u8>>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            read_tree(&path, base, found);
        } else if let Ok(relative) = path.strip_prefix(base) {
            found.insert(
                relative.display().to_string(),
                fs::read(&path).unwrap_or_default(),
            );
        }
    }
}

/// 前後の差を、パスの一覧だけの短い説明にする (中身は出さない)。
fn drift(before: &BTreeMap<String, Vec<u8>>, after: &BTreeMap<String, Vec<u8>>) -> Option<String> {
    let keys_before: BTreeSet<&String> = before.keys().collect();
    let keys_after: BTreeSet<&String> = after.keys().collect();
    let added: Vec<&&String> = keys_after.difference(&keys_before).collect();
    let removed: Vec<&&String> = keys_before.difference(&keys_after).collect();
    let changed: Vec<&&String> = keys_before
        .intersection(&keys_after)
        .filter(|key| before.get(**key) != after.get(**key))
        .collect();
    if added.is_empty() && removed.is_empty() && changed.is_empty() {
        return None;
    }
    Some(format!("追加={added:?} 削除={removed:?} 変更={changed:?}"))
}

/// 再利用する `aidlc-review-brief` の本体には、書込みの呼出しが 1 つも無い。
#[test]
fn the_reused_review_brief_tool_calls_no_write_api() {
    let raw = fs::read_to_string(repo_root().join(".claude/tools/aidlc-review-brief.ts")).unwrap();
    for api in [
        "writeFileSync",
        "appendFileSync",
        "mkdirSync",
        "rmSync",
        "renameSync",
        "unlinkSync",
        "withAuditLock",
    ] {
        assert!(!raw.contains(api), "読取り専用の候補が {api} を呼んでいる");
    }
}

/// 除外操作を併せ持つ側 (`aidlc-testing-posture`) には書込みがある — 境界は空虚ではない。
#[test]
fn the_tool_that_also_owns_an_excluded_operation_really_writes() {
    let raw =
        fs::read_to_string(repo_root().join(".claude/tools/aidlc-testing-posture.ts")).unwrap();
    assert!(
        raw.contains("writeFileSync") && raw.contains("withAuditLock"),
        "承認開始を持つ側に書込みが無い — ファイル名だけの判定になっている"
    );
    for excluded in ["\"begin\"", "\"verify\""] {
        assert!(raw.contains(excluded), "除外操作 {excluded} が見当たらない");
    }
    let reused: Vec<&str> = REUSED
        .iter()
        .filter(|(face, _)| *face == "aidlc-testing-posture")
        .map(|(_, verb)| *verb)
        .collect();
    assert_eq!(reused, vec!["render", "fingerprint"]);
}

/// 5 動詞のどれを回しても、作業記録の正本は 1 バイトも変わらない。
#[tokio::test]
async fn the_five_reused_verbs_leave_every_canonical_file_byte_identical() {
    let workspace = Workspace::minted().await;
    let questions = workspace.root().join("questions.md");
    fs::write(&questions, "# Questions\n\n[Answer]: \n").unwrap();
    let before = workspace.canon();
    assert!(
        before.keys().any(|path| path.ends_with("aidlc-state.md")),
        "鋳造で状態ファイルができていない: {:?}",
        before.keys().collect::<Vec<_>>()
    );
    assert!(
        before.keys().any(|path| path.contains("/audit/")),
        "鋳造で監査シャードができていない"
    );
    for (face, verb) in REUSED {
        let args = arguments(verb, &questions);
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        let output = workspace.tool(face, &borrowed);
        if let Some(drift) = drift(&before, &workspace.canon()) {
            panic!(
                "{face} {verb}: 正本が変わった — {drift} (exit={:?}, stderr={})",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

/// 引数が足りない再利用は、黙って書かずに失敗として返る。
#[tokio::test]
async fn a_reused_verb_that_cannot_run_fails_loudly_and_writes_nothing() {
    let workspace = Workspace::minted().await;
    let before = workspace.canon();
    let output = workspace.tool("aidlc-review-brief", &["summary", "--stage", STAGE]);
    assert_eq!(output.status.code(), Some(1), "失敗を成功へ丸めない");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("questions-file"),
        "失敗の理由を黙らせない: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        drift(&before, &workspace.canon()),
        None,
        "失敗経路で正本へ書いた"
    );
}

/// 未知の動詞は拒否され、正本へ触れない。
#[tokio::test]
async fn an_unknown_verb_on_a_reused_tool_is_refused_without_touching_the_record() {
    let workspace = Workspace::minted().await;
    let before = workspace.canon();
    let output = workspace.tool("aidlc-review-brief", &["persist", "--stage", STAGE]);
    assert_ne!(output.status.code(), Some(0));
    assert_eq!(drift(&before, &workspace.canon()), None);
}

/// 再利用の集合には、C6 が読取り専用から除外する操作が 1 つも入っていない。
#[test]
fn the_reuse_set_excludes_every_operation_c6_excludes_from_read_only() {
    let excluded = [
        // receipt_creation
        ("aidlc-log", "answer"),
        ("aidlc-log", "decision"),
        ("aidlc-log", "review"),
        ("aidlc-log", "link"),
        // code_generation_authority_begin
        ("aidlc-testing-posture", "begin"),
        // learnings_persist
        ("aidlc-learnings", "persist"),
        // audited_catalog_updates
        ("aidlc-knowledge", "onboard"),
        ("aidlc-knowledge", "sync"),
    ];
    for (face, verb) in excluded {
        assert!(
            !REUSED
                .iter()
                .any(|(reused_face, reused_verb)| *reused_face == face && *reused_verb == verb),
            "{face} {verb} は読取り専用の再利用に入れてはならない"
        );
    }
    assert_eq!(REUSED.len(), 5, "C6 の再利用候補は 2 本 5 動詞である");
}
