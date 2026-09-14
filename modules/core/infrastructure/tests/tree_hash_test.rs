//! `tree_hash` の契約 — ディレクトリ木の決定的な内容ハッシュ。
//!
//! upstream `treeGeneration` (`aidlc-lib.ts:1980`) の観測を固定する。codekb の
//! compare-and-swap 世代 (`sha256:<hex>`) と、非 git ワークスペースでの源指紋の後退先が
//! この 1 本に載る。**ドメインを持たない汎用機構**なので、契約はここで閉じる。
#![allow(clippy::unwrap_used, clippy::expect_used)]

use core_infrastructure::tree_hash::hash_tree;
use std::fs;
use std::path::Path;

fn write(root: &Path, relative: &str, body: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

fn owned(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

/// 同じ内容は同じ値、1 バイト違えば別の値。
#[test]
fn the_same_content_hashes_to_the_same_value_and_a_changed_byte_does_not() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    for root in [first.path(), second.path()] {
        write(root, "src/main.rs", "fn main() {}\n");
        write(root, "src/lib.rs", "pub fn f() {}\n");
    }
    let a = hash_tree(first.path(), &owned(&["src/"]), &[]).unwrap();
    let b = hash_tree(second.path(), &owned(&["src/"]), &[]).unwrap();
    assert_eq!(a, b, "作業ディレクトリの絶対パスに依存しない");
    assert_eq!(a.len(), 64, "sha256 の 16 進表記");

    write(second.path(), "src/main.rs", "fn main() { }\n");
    let changed = hash_tree(second.path(), &owned(&["src/"]), &[]).unwrap();
    assert_ne!(a, changed, "内容が変われば値も変わる");
}

/// ファイル名だけが違っても別の値になる (内容の連結ではなく木を畳んでいる)。
#[test]
fn a_renamed_file_changes_the_hash_even_when_the_bytes_are_the_same() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "src/a.rs", "x\n");
    let before = hash_tree(root.path(), &owned(&["src/"]), &[]).unwrap();
    fs::remove_file(root.path().join("src/a.rs")).unwrap();
    write(root.path(), "src/b.rs", "x\n");
    let after = hash_tree(root.path(), &owned(&["src/"]), &[]).unwrap();
    assert_ne!(before, after);
}

/// `.git` は常に除外される (指定しなくても)。
#[test]
fn the_git_directory_is_always_excluded() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "src/main.rs", "fn main() {}\n");
    write(root.path(), ".git/HEAD", "ref: refs/heads/main\n");
    let before = hash_tree(root.path(), &owned(&["./"]), &[]).unwrap();
    write(root.path(), ".git/HEAD", "ref: refs/heads/other\n");
    let after = hash_tree(root.path(), &owned(&["./"]), &[]).unwrap();
    assert_eq!(before, after, ".git の中身は畳まない");
}

/// 呼び手が指定した除外は、その配下ごと畳まない。
#[test]
fn a_caller_supplied_exclusion_removes_the_subtree() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "src/main.rs", "fn main() {}\n");
    write(root.path(), "aidlc/state.md", "one\n");
    let before = hash_tree(root.path(), &owned(&["./"]), &owned(&["aidlc"])).unwrap();
    write(root.path(), "aidlc/state.md", "two\n");
    let after = hash_tree(root.path(), &owned(&["./"]), &owned(&["aidlc"])).unwrap();
    assert_eq!(before, after);

    // 除外しなければ同じ編集で値が動く — 上の一致が「畳んでいないから」だと示す。
    let unexcluded_before = hash_tree(root.path(), &owned(&["./"]), &[]).unwrap();
    write(root.path(), "aidlc/state.md", "three\n");
    let unexcluded_after = hash_tree(root.path(), &owned(&["./"]), &[]).unwrap();
    assert_ne!(unexcluded_before, unexcluded_after);
}

/// 同じパスを 2 度渡しても 1 度と同じ (upstream の `new Set` と、訪問済みの記憶)。
#[test]
fn repeating_a_path_does_not_change_the_hash() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "src/main.rs", "fn main() {}\n");
    let once = hash_tree(root.path(), &owned(&["src/"]), &[]).unwrap();
    let twice = hash_tree(root.path(), &owned(&["src/", "src/"]), &[]).unwrap();
    assert_eq!(once, twice);
}

/// 畳めない指定は **`None`** — 偽の値も空木の値も返さない。
#[test]
fn an_unusable_specification_yields_none_instead_of_a_false_value() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "src/main.rs", "fn main() {}\n");
    assert_eq!(hash_tree(root.path(), &[], &[]), None, "パスが空");
    assert_eq!(
        hash_tree(root.path(), &owned(&["no-such-dir/"]), &[]),
        None,
        "在らないパス"
    );
    assert_eq!(
        hash_tree(root.path(), &owned(&["../outside"]), &[]),
        None,
        "根の外へ出る"
    );
    assert_eq!(
        hash_tree(root.path(), &owned(&["/etc"]), &[]),
        None,
        "絶対パス"
    );
    assert_eq!(
        hash_tree(root.path(), &owned(&["src/*.rs"]), &[]),
        None,
        "glob 文字"
    );
    assert_eq!(hash_tree(root.path(), &owned(&[""]), &[]), None, "空の綴り");
}

/// シンボリックリンクは**辿らず**、指し先の綴りとして畳む。
#[cfg(unix)]
#[test]
fn a_symlink_is_folded_as_its_target_spelling_and_never_followed() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "src/main.rs", "fn main() {}\n");
    std::os::unix::fs::symlink("main.rs", root.path().join("src/link.rs")).unwrap();
    let before = hash_tree(root.path(), &owned(&["src/"]), &[]).unwrap();
    fs::remove_file(root.path().join("src/link.rs")).unwrap();
    std::os::unix::fs::symlink("other.rs", root.path().join("src/link.rs")).unwrap();
    let after = hash_tree(root.path(), &owned(&["src/"]), &[]).unwrap();
    assert_ne!(before, after, "指し先の綴りが値に入る");
}

// ---------------------------------------------------------------------------
// upstream `treeGeneration` とのバイト一致 (固定ピン a277af21 の実測値)
// ---------------------------------------------------------------------------
//
// 自己整合だけでは「同じ規則で畳んでいる」ことしか言えない。畳む順序・区切りバイト・
// 種類の前置きが upstream と 1 バイトでもずれたら codekb の compare-and-swap が噛み合わない
// ので、ピンの配布ツール (`aidlc-utility.ts codekb-snapshot` / `codekb-publish`) を実際に
// 走らせて採った 3 つの値をここに焼き込む。値の採り直しは
// `bun scripts/goldens/capture-codekb-write.ts` の採取と同じ木を建てて行う。

/// 9 成果物の綴り (upstream `CODEKB_ARTIFACT_FILES` 逐語)。
const NINE: [&str; 9] = [
    "api-documentation.md",
    "architecture.md",
    "business-overview.md",
    "code-quality-assessment.md",
    "code-structure.md",
    "component-inventory.md",
    "dependencies.md",
    "reverse-engineering-timestamp.md",
    "technology-stack.md",
];

/// 走査範囲ブロック付きの鮮度印 (採取に使った本文と 1 バイトも違えない)。
fn timestamp_body(fingerprint: &str) -> String {
    format!(
        "# Reverse Engineering Timestamp\n\n## Scope of Analysis\n\n```yaml\nscope_version: 1\nkind: partial\nintent: demo-intent\nfingerprint: {fingerprint}\nanalyzed:\n  paths:\n    - src/\n  components:\n    - alpha\n```\n"
    )
}

/// 9 成果物を 1 ディレクトリへ書く。
fn write_nine(dir: &Path, flavour: &str) {
    for name in NINE {
        let body = if name == "reverse-engineering-timestamp.md" {
            timestamp_body("3c55f6af3e88b8f5f11eeb1eafeeb84095e998db")
        } else {
            format!("# {name}\n{flavour}\n")
        };
        write(dir, name, &body);
    }
}

/// 源の指紋の後退先 — 非 git ワークスペースで `src/` だけを畳んだ値。
#[test]
fn the_non_git_source_fallback_matches_the_upstream_observation() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "src/main.rs", "fn main() {}\n");
    write(
        root.path(),
        "aidlc/spaces/default/intents/260101-demo-aaaaaaaa/aidlc-state.md",
        "# AI-DLC State\n- **Project**: p\n",
    );
    assert_eq!(
        hash_tree(root.path(), &owned(&["src/"]), &owned(&["aidlc"])).unwrap(),
        "8ab3c569fa3d1860a8d5e3fc244c959001f2bc6a94266acc7cfdfc05d32d9baa",
        "upstream codekb-snapshot --json の source_fingerprint (tree: の後ろ) と一致すること"
    );
}

/// ストア世代 — 既存の 9 成果物 (`old`) を丸ごと畳んだ値。
#[test]
fn the_existing_store_generation_matches_the_upstream_observation() {
    let root = tempfile::tempdir().unwrap();
    let store = root.path().join("codekb");
    fs::create_dir_all(&store).unwrap();
    write_nine(&store, "old");
    assert_eq!(
        hash_tree(&store, &owned(&["./"]), &[]).unwrap(),
        "d36c0993a88f26782c44b7ab1c24cb21ea1300b2465e21ec6b5d32d6190cff3f",
        "upstream codekb-snapshot の STORE_GENERATION (sha256: の後ろ) と一致すること"
    );
}

/// 公開後の世代 — 差し替えた 9 成果物 (`new`) を丸ごと畳んだ値。
#[test]
fn the_published_store_generation_matches_the_upstream_observation() {
    let root = tempfile::tempdir().unwrap();
    let store = root.path().join("codekb");
    fs::create_dir_all(&store).unwrap();
    write_nine(&store, "new");
    assert_eq!(
        hash_tree(&store, &owned(&["./"]), &[]).unwrap(),
        "f19143e206ae316ebe72c429b40ed204db041e915a9bcfab0c138d906c338dd2",
        "upstream codekb-publish の PUBLISHED 行の世代と一致すること"
    );
}
