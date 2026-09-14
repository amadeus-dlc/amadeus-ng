//! bugfix スコープ 1 周が踏む配布側の入口の列挙 (`scripts/aidlc-selfhost/required-surface.json`) と、
//! この build の配線状態の突合 (FR5 / C6)。
//!
//! 列挙は配布資産の**実バイト**から導く。数え方の取り違え —
//! 未配線の動詞を配線済みと数える、配布 TypeScript のまま再利用する動詞を native と数える、
//! 禁止の記述を呼出しと数える — を、出典行と `aidlc::cli::parse` の実挙動で潰す。
//!
//! 未配線の動詞をここで実装することはしない。C6 は writer を U2、integrator を U4 と定めており、
//! U4 は接続と不足の明示に留まる。
// 契約テストは固定の添字参照と panic を検証の合図として使う (既存の契約テストと同じ許容)。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use aidlc::cli::{Face, Request, parse};

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;

/// この build のバイナリを `target` の名前で置く — **シンボリックリンク**で。
///
/// 実体のリンク数を動かさないので、同じ実体を並列に実行している別のテストを巻き込まない
/// (U3 の `tdd-logs/16-flaky-recheck.log` の同種の不安定さを避ける)。
fn place_binary(target: &std::path::Path) {
    let source = env!("CARGO_BIN_EXE_aidlc");
    #[cfg(unix)]
    {
        if std::os::unix::fs::symlink(source, target).is_ok() {
            return;
        }
    }
    fs::copy(source, target).map(|_| ()).unwrap();
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn surface() -> serde_json::Value {
    let path = repo_root().join("scripts/aidlc-selfhost/required-surface.json");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("必要集合が読めない ({}): {error}", path.display()));
    serde_json::from_str(&raw).expect("必要集合は JSON")
}

fn entries(surface: &serde_json::Value) -> Vec<&serde_json::Value> {
    surface["verbs"].as_array().expect("verbs").iter().collect()
}

fn text(entry: &serde_json::Value, key: &str) -> String {
    entry[key]
        .as_str()
        .unwrap_or_else(|| panic!("{key} が無い: {entry}"))
        .to_string()
}

/// `aidlc-<face>` から面の短い綴り (`orchestrate` 等) を取る。
fn short_face(face: &str) -> String {
    face.strip_prefix("aidlc-").unwrap_or(face).to_string()
}

/// 出典 `path:line` の行本文。
fn cited_line(cite: &str) -> String {
    let (path, number) = cite.rsplit_once(':').expect("出典は path:line");
    let line: usize = number.parse().expect("行番号");
    let raw = fs::read_to_string(repo_root().join(path))
        .unwrap_or_else(|error| panic!("出典が読めない ({path}): {error}"));
    raw.lines()
        .nth(line - 1)
        .unwrap_or_else(|| panic!("{cite}: その行が無い"))
        .to_string()
}

/// この build の入口が、その面・動詞を拒否するか。
fn refuses(face: &str, verb: &str) -> bool {
    let argv = vec![verb.to_string()];
    matches!(
        parse(Face::of(face), &argv),
        Request::UnknownOrchestrateVerb { .. }
            | Request::UnknownUtilityVerb { .. }
            | Request::UnknownLogVerb { .. }
            | Request::StateNotWired { .. }
            | Request::UnknownStateVerb { .. }
            | Request::BoltNotWired { .. }
            | Request::UnknownBoltVerb { .. }
            | Request::UnknownLearningsVerb { .. }
    )
}

/// argv の写像だけでは受理/拒否が決まらない面 (引数をそのまま運ぶ面)。
fn decided_after_parse(face: &str) -> bool {
    matches!(Face::of(face), Face::Jump | Face::TestingPosture)
}

/// `argv[0]` を変えて 1 回だけ起動し、標準エラーを取る (空のワークスペース)。
///
/// 出力は**ファイルへ**落とし、待つのは直接の子だけにする。この build はセッション補助の
/// 子を起こすことがあり (`aidlc/.aidlc-sessions/pids/`)、パイプの終端を待つ形 (`output()`) は
/// その孫に引きずられうるためである。
fn stderr_of(face: &str, args: &[&str]) -> String {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("workspace");
    fs::create_dir_all(root.join("aidlc/spaces/default/memory")).unwrap();
    fs::create_dir_all(root.join(".claude")).unwrap();
    let binary = temp.path().join(face);
    place_binary(&binary);
    let out = temp.path().join("stdout.txt");
    let err = temp.path().join("stderr.txt");
    let mut child = Command::new(binary)
        .args(args)
        .current_dir(&root)
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", temp.path())
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "C.UTF-8")
        .env("LC_ALL", "C.UTF-8")
        .env("TZ", "UTC")
        .stdin(Stdio::null())
        .stdout(Stdio::from(fs::File::create(&out).unwrap()))
        .stderr(Stdio::from(fs::File::create(&err).unwrap()))
        .spawn()
        .unwrap();
    child.wait().unwrap();
    fs::read_to_string(&err).unwrap_or_default()
}

/// 列挙した呼出しは、すべて配布資産の実バイトから引ける。
#[test]
fn every_listed_invocation_is_quoted_from_the_distribution_bytes() {
    let surface = surface();
    let sources: BTreeSet<String> = surface["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_string())
        .collect();
    for entry in entries(&surface) {
        let face = text(entry, "face");
        let verb = text(entry, "verb");
        let cites = entry["cites"].as_array().unwrap();
        assert!(!cites.is_empty(), "{face} {verb}: 出典が無い");
        for cite in cites {
            let cite = cite.as_str().unwrap();
            let path = cite.rsplit_once(':').unwrap().0;
            assert!(sources.contains(path), "{cite}: 列挙の対象外のファイル");
            let line = cited_line(cite);
            let token = text(entry, "cite_token");
            assert!(
                line.contains(&token),
                "{cite}: その行に {token} が無い — {line}"
            );
        }
        // 呼出し形の出典は、逐語の起動行でなければならない。
        for cite in entry["invoked_cites"].as_array().unwrap() {
            let cite = cite.as_str().unwrap();
            let line = cited_line(cite);
            let runnable = format!(".claude/tools/{face}.ts {verb}");
            assert!(
                line.contains(&runnable) && line.contains("bun "),
                "{cite}: 呼出し形ではない — {line}"
            );
        }
    }
}

/// 列挙は、対象ファイルが語る面・動詞の組を取りこぼしても増やしてもいない。
#[test]
fn the_enumeration_matches_the_face_and_verb_pairs_the_bugfix_bytes_mention() {
    let surface = surface();
    let listed: BTreeSet<(String, String)> = entries(&surface)
        .iter()
        .map(|entry| (short_face(&text(entry, "face")), text(entry, "verb")))
        .collect();
    let mut found: BTreeSet<(String, String)> = BTreeSet::new();
    for source in surface["sources"].as_array().unwrap() {
        let path = source.as_str().unwrap();
        let raw = fs::read_to_string(repo_root().join(path)).unwrap();
        for (face, verb) in mentions(&raw) {
            found.insert((face, verb));
        }
    }
    // `intent-create` と `--doctor` は綴りが動詞形と違うので、機械抽出の対象外である。
    let spelled_apart: BTreeSet<(String, String)> = [
        ("utility".to_string(), "intent-create".to_string()),
        ("orchestrate".to_string(), "--doctor".to_string()),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        listed,
        found
            .union(&spelled_apart)
            .cloned()
            .collect::<BTreeSet<_>>(),
        "列挙と実バイトの面・動詞の組がずれている"
    );
}

/// `aidlc-<face>.ts <verb>` の言及を拾う (出典抽出と同じ規則)。
fn mentions(raw: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for line in raw.lines() {
        let mut rest = line;
        while let Some(start) = rest.find("aidlc-") {
            let tail = &rest[start..];
            let after = &tail["aidlc-".len()..];
            let face_len = after
                .find(|c: char| !(c.is_ascii_lowercase() || c == '-'))
                .unwrap_or(after.len());
            let face = &after[..face_len];
            let remainder = &after[face_len..];
            if let Some(stripped) = remainder.strip_prefix(".ts") {
                let trimmed = stripped.trim_start_matches([' ', '\t']);
                if trimmed.len() < stripped.len() {
                    let verb_len = trimmed
                        .find(|c: char| !(c.is_ascii_lowercase() || c == '-'))
                        .unwrap_or(trimmed.len());
                    let verb = &trimmed[..verb_len];
                    if !verb.is_empty() && !face.is_empty() {
                        found.push((face.to_string(), verb.to_string()));
                    }
                }
            }
            rest = &tail[1..];
        }
    }
    found
}

/// `wired` と書いた動詞は、この build の入口が実際に受理する。
#[test]
fn a_verb_marked_wired_is_accepted_by_this_build() {
    let surface = surface();
    for entry in entries(&surface) {
        if text(entry, "status") != "wired" {
            continue;
        }
        let face = text(entry, "face");
        let verb = text(entry, "verb");
        if decided_after_parse(&face) {
            // 引数をそのまま運ぶ面は、実行してはじめて受理/拒否が決まる。
            let stderr = stderr_of(&face, &[&verb]);
            assert!(
                !stderr.contains("Unknown subcommand") && !stderr.contains("is not connected"),
                "{face} {verb}: 受理されていない — {stderr}"
            );
            continue;
        }
        assert!(!refuses(&face, &verb), "{face} {verb}: 拒否された");
    }
}

/// `not-wired` と書いた動詞は、この build が実際に拒否する。
#[test]
fn a_verb_marked_not_wired_is_refused_by_this_build() {
    let surface = surface();
    let mut checked = 0;
    for entry in entries(&surface) {
        if text(entry, "status") != "not-wired" {
            continue;
        }
        let face = text(entry, "face");
        let verb = text(entry, "verb");
        assert!(
            !decided_after_parse(&face),
            "{face}: 未配線の判定を実行時へ先送りしない"
        );
        assert!(refuses(&face, &verb), "{face} {verb}: 拒否されていない");
        checked += 1;
    }
    assert!(checked > 0, "未配線の動詞が 1 件も無い — 列挙が空である");
}

/// `distributed-ts` と書いた動詞は、この build に入口が無く、配布 TypeScript が現物にある。
#[test]
fn a_verb_marked_distributed_ts_has_no_native_entry_point_and_keeps_its_tool() {
    let surface = surface();
    let reused: BTreeSet<String> = entries(&surface)
        .iter()
        .filter(|entry| text(entry, "status") == "distributed-ts")
        .map(|entry| format!("{} {}", text(entry, "face"), text(entry, "verb")))
        .collect();
    for entry in entries(&surface) {
        if text(entry, "status") != "distributed-ts" {
            continue;
        }
        let face = text(entry, "face");
        let verb = text(entry, "verb");
        assert!(refuses(&face, &verb), "{face} {verb}: native の入口がある");
        let tool = repo_root().join(".claude/tools").join(format!("{face}.ts"));
        assert!(tool.is_file(), "{}: 配布 TypeScript が無い", tool.display());
    }
    // C6 の `reader_candidates` のうち、この build に入口が無いのは review-brief の 3 動詞である。
    // `testing-posture` の 2 動詞は native が持つので、再利用の対象には数えない。
    assert_eq!(
        reused,
        [
            "aidlc-review-brief context",
            "aidlc-review-brief review",
            "aidlc-review-brief summary",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
    );
}

/// フックの列挙は、接続定義 (`hook-binding.json`) と 1 対 1 で一致する。
#[test]
fn the_hook_enumeration_agrees_with_the_binding_definition() {
    let surface = surface();
    let mut listed: BTreeMap<String, String> = BTreeMap::new();
    for hook in surface["hooks"].as_array().unwrap() {
        listed.insert(text(hook, "name"), text(hook, "status"));
    }
    let raw =
        fs::read_to_string(repo_root().join("scripts/aidlc-selfhost/hook-binding.json")).unwrap();
    let binding: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let mut expected: BTreeMap<String, String> = BTreeMap::new();
    for name in binding["native_hooks"].as_array().unwrap() {
        expected.insert(
            name.as_str().unwrap().to_string(),
            "native-available".to_string(),
        );
    }
    for entry in binding["distributed_hooks"].as_array().unwrap() {
        expected.insert(
            entry["name"].as_str().unwrap().to_string(),
            "distributed-only".to_string(),
        );
    }
    assert_eq!(listed, expected, "フックの分類が 2 つの資料でずれている");
}

/// bugfix 一周で必ず踏む動詞は、配布側に呼出し形があり、状態が実挙動と一致する。
///
/// ここは「不足の明示」そのものである — 必ず踏むのに `not-wired` の動詞は U2 の責任として
/// 残り、U4 では実装しない。だからこそ、その分類が取り違えでないことを実挙動で確かめる。
#[test]
fn every_verb_the_bugfix_loop_requires_is_backed_by_a_runnable_citation_and_a_measured_status() {
    let surface = surface();
    let mut required = 0;
    let mut gaps: BTreeSet<String> = BTreeSet::new();
    for entry in entries(&surface) {
        if !entry["bugfix_required"].as_bool().unwrap_or(false) {
            continue;
        }
        required += 1;
        let face = text(entry, "face");
        let verb = text(entry, "verb");
        let runnable = entry
            .get("runnable_source")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        assert!(
            text(entry, "usage") == "invoked" || !runnable.is_empty(),
            "{face} {verb}: 必ず踏むと書きながら、呼出し形も起動形の出所も無い"
        );
        assert!(
            !text(entry, "condition").is_empty(),
            "{face} {verb}: 条件の記述が無い"
        );
        if text(entry, "status") == "not-wired" {
            assert!(
                refuses(&face, &verb),
                "{face} {verb}: 未配線と書きながら受理された"
            );
            gaps.insert(format!("{face} {verb}"));
        }
    }
    assert!(required > 0, "必ず踏む動詞が 1 件も無い");
    // 不足は隠さず、この build の実挙動で裏取りされた一覧として残す。
    for gap in &gaps {
        let (face, verb) = gap.split_once(' ').unwrap();
        assert!(refuses(face, verb), "{gap}");
    }
}
