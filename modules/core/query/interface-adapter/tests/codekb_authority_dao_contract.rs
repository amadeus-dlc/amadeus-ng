//! 群 B/C の読取 DAO とユースケースの契約 — **走査範囲・登録簿・作業ツリーを鍵で読み、
//! 当たらなければ観測として返す**。
//!
//! ここが固定するのは:
//! 1. `CodekbScopeDaoImpl` は fenced yaml の走査範囲ブロックを解き、ブロックの不在 (absent) と
//!    綴りの破れ (malformed) を**読取失敗ではなく観測**として返す。ファイルの不在は `None`。
//! 2. `IntentReposDaoImpl` は `dirName` の逐語一致と legacy の `<slug>-<id8>` 一致で行を引き、
//!    登録簿の不在・破損はどちらも空に畳む。
//! 3. `CodekbSourceFingerprintDaoImpl` は作業ツリーの内容指紋を返し、非 git・不一致 pathspec・
//!    全除外をすべて `None` に畳む (偽の判定も空ツリーの指紋も返さない)。
//! 4. `ResolveCodekbRepoUseCase` は 名指し > 唯一の記録 > ワークスペース名 の順で決める。
//! 5. `DiffCodekbScopeUseCase` は 6 判定を、`CompareCodekbScopeUseCase` は覆いの判定を組む。
#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::fs;
use std::path::Path;
use std::process::Command;

use core_query_interface_adapter::{
    CodekbScopeDaoImpl, CodekbSourceFingerprintDaoImpl, IntentReposDaoImpl,
};
use core_query_use_case::orchestration::{
    CodekbScopeDao, CodekbScopeDiffView, CodekbSourceFingerprintDao, CompareCodekbScopeError,
    CompareCodekbScopeUseCase, DiffCodekbScopeUseCase, IntentReposDao,
    MintCodekbFingerprintUseCase, ReScopeParseView, ResolveCodekbRepoUseCase,
};

/// 走査範囲ブロックを持つ鮮度印を組み立てる。
fn timestamp(block: &str) -> String {
    format!("# Reverse Engineering Timestamp\n\n## Scope of Analysis\n\n```yaml\n{block}\n```\n")
}

fn write(root: &Path, relative: &str, body: &str) -> std::path::PathBuf {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("親")).expect("親ディレクトリ");
    fs::write(&path, body).expect("ファイル");
    path
}

/// 決定的な git リポジトリを建てる (開発者の global gitignore と改行変換を無効化する)。
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
        assert!(output.status.success(), "git {args:?}");
    }
}

fn paths(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

// --- 1. 走査範囲ブロックの読取 ------------------------------------------------

#[test]
fn an_absent_timestamp_file_is_not_a_failure() {
    let root = tempfile::tempdir().expect("一時ディレクトリ");
    let dao = CodekbScopeDaoImpl::new(&root.path().join("reverse-engineering-timestamp.md"));
    assert_eq!(dao.find(), Ok(None));
}

#[test]
fn a_scope_block_is_parsed_with_every_list() {
    let root = tempfile::tempdir().expect("一時ディレクトリ");
    let path = write(
        root.path(),
        "ts.md",
        &timestamp(
            "scope_version: 1\nkind: partial\nintent: fix-payment\nfingerprint: abc123\nanalyzed:\n  paths:\n    - src/payments/\n  components:\n    - payment-gateway\nshallow:\n  paths:\n    - src/",
        ),
    );

    let parsed = CodekbScopeDaoImpl::new(&path)
        .find()
        .expect("読取")
        .expect("在る");

    let ReScopeParseView::Parsed(scope) = parsed else {
        panic!("解けるはず: {parsed:?}");
    };
    assert_eq!(scope.kind(), "partial");
    assert_eq!(scope.intent(), "fix-payment");
    assert_eq!(scope.fingerprint(), Some("abc123"));
    assert_eq!(scope.analyzed_paths(), paths(&["src/payments/"]).as_slice());
    assert_eq!(
        scope.analyzed_components(),
        paths(&["payment-gateway"]).as_slice()
    );
    assert_eq!(scope.shallow_paths(), paths(&["src/"]).as_slice());
}

/// `unknown` と空欄はどちらも「指紋を記録していない」である。
#[test]
fn an_unknown_or_blank_fingerprint_reads_as_absent() {
    let root = tempfile::tempdir().expect("一時ディレクトリ");
    for (name, line) in [("a.md", "fingerprint: unknown"), ("b.md", "fingerprint:")] {
        let path = write(
            root.path(),
            name,
            &timestamp(&format!(
                "scope_version: 1\nkind: partial\nintent: x\n{line}\nanalyzed:\n  paths:\n    - src/"
            )),
        );
        let ReScopeParseView::Parsed(scope) = CodekbScopeDaoImpl::new(&path)
            .find()
            .expect("読取")
            .expect("在る")
        else {
            panic!("解けるはず");
        };
        assert_eq!(scope.fingerprint(), None, "{name}");
    }
}

/// ブロックが無い・綴りが破れているのは**読取失敗ではなく観測**である。
#[test]
fn an_unreadable_scope_block_is_an_observation_not_a_failure() {
    let root = tempfile::tempdir().expect("一時ディレクトリ");
    let absent = write(root.path(), "absent.md", "# ts\n\n散文しかない\n");
    assert!(matches!(
        CodekbScopeDaoImpl::new(&absent).find().expect("読取"),
        Some(ReScopeParseView::Absent(_))
    ));

    for (name, block, expected) in [
        (
            "version.md",
            "scope_version: 2\nkind: partial",
            "unknown scope_version: 2",
        ),
        (
            "kind.md",
            "scope_version: 1\nkind: shallow",
            "kind must be full|partial, got: shallow",
        ),
        (
            "nokind.md",
            "scope_version: 1\nintent: x\nanalyzed:\n  paths:\n    - src/",
            "missing kind: line",
        ),
        (
            "nopaths.md",
            "scope_version: 1\nkind: partial\nintent: x",
            "kind: partial requires analyzed.paths entries",
        ),
        (
            "rootpartial.md",
            "scope_version: 1\nkind: partial\nintent: x\nanalyzed:\n  paths:\n    - ./",
            "repository-root coverage (./) requires kind: full",
        ),
        (
            "fullnoroot.md",
            "scope_version: 1\nkind: full\nintent: x\nanalyzed:\n  paths:\n    - src/",
            "kind: full requires repository-root coverage (analyzed.paths must include ./)",
        ),
    ] {
        let path = write(root.path(), name, &timestamp(block));
        let parsed = CodekbScopeDaoImpl::new(&path)
            .find()
            .expect("読取")
            .expect("在る");
        let ReScopeParseView::Malformed(detail) = parsed else {
            panic!("{name}: malformed のはず: {parsed:?}");
        };
        assert_eq!(detail, expected, "{name}");
    }
}

/// 走査範囲でない yaml ブロックは読み飛ばし、`scope_version` を持つブロックを探し当てる。
#[test]
fn a_non_scope_yaml_block_is_skipped() {
    let root = tempfile::tempdir().expect("一時ディレクトリ");
    let path = write(
        root.path(),
        "ts.md",
        "```yaml\nunrelated: true\n```\n\n```yaml\nscope_version: 1\nkind: partial\nintent: x\nanalyzed:\n  paths:\n    - src/\n```\n",
    );
    assert!(matches!(
        CodekbScopeDaoImpl::new(&path).find().expect("読取"),
        Some(ReScopeParseView::Parsed(_))
    ));
}

// --- 2. intent 登録簿 ---------------------------------------------------------

#[test]
fn the_registry_is_matched_by_dir_name_and_falls_back_to_the_legacy_shape() {
    let root = tempfile::tempdir().expect("一時ディレクトリ");
    let registry = write(
        root.path(),
        "intents.json",
        r#"[
          {"uuid":"11111111-2222-4333-8444-555555555555","slug":"demo","dirName":"260101-demo-aaaaaaaa","repos":["svc-a"],"status":"active"},
          {"uuid":"aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeee1234","slug":"legacy","repos":["svc-b","svc-c"],"status":"active"}
        ]"#,
    );
    let dao = IntentReposDaoImpl::new(&registry);

    assert_eq!(dao.find("260101-demo-aaaaaaaa"), paths(&["svc-a"]));
    // `dirName` を持たない行は `<slug>-<uuid 末尾 hex>` で引く。
    assert_eq!(dao.find("legacy-eeeeeeee1234"), paths(&["svc-b", "svc-c"]));
    assert!(dao.find("no-such-record").is_empty());
}

/// 登録簿の不在・破損はどちらも空に畳む（拒否に変えない）。
#[test]
fn an_absent_or_broken_registry_reads_as_no_recorded_repos() {
    let root = tempfile::tempdir().expect("一時ディレクトリ");
    assert!(
        IntentReposDaoImpl::new(&root.path().join("intents.json"))
            .find("x")
            .is_empty()
    );
    let broken = write(root.path(), "broken.json", "{not an array");
    assert!(IntentReposDaoImpl::new(&broken).find("x").is_empty());
}

// --- 3. 作業ツリーの内容指紋 --------------------------------------------------

#[test]
fn the_fingerprint_is_content_addressed_and_collapses_every_failure_to_none() {
    let root = tempfile::tempdir().expect("一時ディレクトリ");
    let workspace = root.path().join("demo");
    write(&workspace, "src/main.rs", "fn main() {}\n");
    write(
        &workspace,
        "aidlc/spaces/default/intents/x/aidlc-state.md",
        "# s\n",
    );

    // 非 git ではまだ計算できない。
    let dao = CodekbSourceFingerprintDaoImpl::new(&workspace, vec!["aidlc".to_string()]);
    assert_eq!(dao.find(&paths(&["src/"])), None, "非 git は unknown");

    git_init(&workspace);
    let first = dao.find(&paths(&["src/"])).expect("git なら計算できる");
    assert_eq!(first.len(), 40, "git write-tree は 40 桁 hex");
    assert_eq!(dao.find(&paths(&["src/"])), Some(first.clone()), "決定的");

    // 空の一覧・不一致 pathspec・全除外はすべて None へ畳む。
    assert_eq!(dao.find(&[]), None);
    assert_eq!(dao.find(&paths(&["no-such-dir/"])), None);
    assert_eq!(
        dao.find(&paths(&["aidlc/"])),
        None,
        "除外だけなら計算しない"
    );

    // 除外は全体指紋の内側で効く — `aidlc/` を書いても `./` の指紋は動かない。
    let whole = dao.find(&paths(&["./"])).expect("全体");
    write(
        &workspace,
        "aidlc/spaces/default/intents/x/churn.md",
        "churn\n",
    );
    assert_eq!(dao.find(&paths(&["./"])), Some(whole), "aidlc/ は除かれる");

    // ソースを触れば動く。
    write(&workspace, "src/main.rs", "fn main() { /* changed */ }\n");
    assert_ne!(dao.find(&paths(&["src/"])), Some(first));
}

#[test]
fn the_mint_use_case_passes_the_paths_through() {
    let root = tempfile::tempdir().expect("一時ディレクトリ");
    let workspace = root.path().join("demo");
    write(&workspace, "src/main.rs", "fn main() {}\n");
    git_init(&workspace);

    let minted = MintCodekbFingerprintUseCase::new(CodekbSourceFingerprintDaoImpl::new(
        &workspace,
        Vec::new(),
    ))
    .execute(&paths(&["src/"]));

    assert!(minted.is_some_and(|hash| hash.len() == 40));
}

// --- 4. リポジトリ名の解決 ----------------------------------------------------

#[test]
fn the_repo_name_prefers_the_request_then_the_lone_record_then_the_workspace() {
    let root = tempfile::tempdir().expect("一時ディレクトリ");
    let one = write(
        root.path(),
        "one.json",
        r#"[{"uuid":"1","slug":"demo","dirName":"rec","repos":["svc-a"],"status":"active"}]"#,
    );
    let two = write(
        root.path(),
        "two.json",
        r#"[{"uuid":"1","slug":"demo","dirName":"rec","repos":["svc-a","svc-b"],"status":"active"}]"#,
    );

    let lone = ResolveCodekbRepoUseCase::new(IntentReposDaoImpl::new(&one));
    assert_eq!(lone.execute(Some("named"), Some("rec"), "ws"), "named");
    assert_eq!(lone.execute(None, Some("rec"), "ws"), "svc-a");
    // 空文字は名指しに数えない。
    assert_eq!(lone.execute(Some(""), Some("rec"), "ws"), "svc-a");
    // record が無ければワークスペース名。
    assert_eq!(lone.execute(None, None, "ws"), "ws");

    let ambiguous = ResolveCodekbRepoUseCase::new(IntentReposDaoImpl::new(&two));
    assert_eq!(
        ambiguous.execute(None, Some("rec"), "ws"),
        "ws",
        "2 つ以上なら決められないのでワークスペース名へ後退する"
    );
}

// --- 5. 判定の組み立て --------------------------------------------------------

/// ストアと作業ツリーを据えて status を引く。
fn status_of(workspace: &Path, block: Option<&str>) -> CodekbScopeDiffView {
    let store = workspace.join("store/reverse-engineering-timestamp.md");
    if let Some(block) = block {
        fs::create_dir_all(store.parent().expect("親")).expect("親");
        fs::write(&store, timestamp(block)).expect("ストア");
    }
    DiffCodekbScopeUseCase::new(
        CodekbScopeDaoImpl::new(&store),
        CodekbSourceFingerprintDaoImpl::new(workspace, vec!["store".to_string()]),
    )
    .execute()
    .expect("読取")
}

#[test]
fn the_status_verdicts_cover_absence_freshness_and_unverifiability() {
    let root = tempfile::tempdir().expect("一時ディレクトリ");
    let workspace = root.path().join("demo");
    write(&workspace, "src/main.rs", "fn main() {}\n");
    git_init(&workspace);

    assert_eq!(status_of(&workspace, None), CodekbScopeDiffView::NoStore);

    let current = CodekbSourceFingerprintDaoImpl::new(&workspace, vec!["store".to_string()])
        .find(&paths(&["src/"]))
        .expect("指紋");

    let fresh = status_of(
        &workspace,
        Some(&format!(
            "scope_version: 1\nkind: partial\nintent: store-intent\nfingerprint: {current}\nanalyzed:\n  paths:\n    - src/"
        )),
    );
    assert!(
        matches!(fresh, CodekbScopeDiffView::Current { ref store_fingerprint, .. } if *store_fingerprint == current),
        "{fresh:?}"
    );

    let stale = status_of(
        &workspace,
        Some(
            "scope_version: 1\nkind: partial\nintent: store-intent\nfingerprint: 1111111111111111111111111111111111111111\nanalyzed:\n  paths:\n    - src/",
        ),
    );
    assert!(
        matches!(stale, CodekbScopeDiffView::Stale { .. }),
        "{stale:?}"
    );

    let no_fingerprint = status_of(
        &workspace,
        Some(
            "scope_version: 1\nkind: partial\nintent: x\nfingerprint: unknown\nanalyzed:\n  paths:\n    - src/",
        ),
    );
    assert!(
        matches!(
            no_fingerprint,
            CodekbScopeDiffView::UnverifiedWithoutFingerprint { .. }
        ),
        "{no_fingerprint:?}"
    );

    let uncomputable = status_of(
        &workspace,
        Some(
            "scope_version: 1\nkind: partial\nintent: x\nfingerprint: 1111111111111111111111111111111111111111\nanalyzed:\n  paths:\n    - no-such-dir/",
        ),
    );
    assert!(
        matches!(
            uncomputable,
            CodekbScopeDiffView::UnverifiedNotComputable { .. }
        ),
        "{uncomputable:?}"
    );

    let absent_block = status_of(&workspace, Some("unrelated: true"));
    assert!(
        matches!(absent_block, CodekbScopeDiffView::StoreScopeAbsent { .. }),
        "{absent_block:?}"
    );
}

/// 突合は覆いを見て、失う主張を名指す。
#[test]
fn the_comparison_names_what_an_overwrite_would_discard() {
    let root = tempfile::tempdir().expect("一時ディレクトリ");
    let store = write(
        root.path(),
        "store.md",
        &timestamp(
            "scope_version: 1\nkind: partial\nintent: store-intent\nfingerprint: aaaa\nanalyzed:\n  paths:\n    - src/payments/\n    - src/auth/\n  components:\n    - payment-gateway\n    - auth-service",
        ),
    );
    let covers = write(
        root.path(),
        "covers.md",
        &timestamp(
            "scope_version: 1\nkind: partial\nintent: incoming\nanalyzed:\n  paths:\n    - src/\n  components:\n    - payment-gateway\n    - auth-service",
        ),
    );
    let narrower = write(
        root.path(),
        "narrower.md",
        &timestamp(
            "scope_version: 1\nkind: partial\nintent: incoming\nanalyzed:\n  paths:\n    - src/payments/\n  components:\n    - payment-gateway",
        ),
    );

    let compare = |incoming: &Path| {
        CompareCodekbScopeUseCase::new(
            CodekbScopeDaoImpl::new(&store),
            CodekbScopeDaoImpl::new(incoming),
        )
        .execute()
    };

    // ディレクトリの接頭辞が両方を飲み込む。
    assert!(matches!(
        compare(&covers).expect("突合"),
        CodekbScopeDiffView::Covers { .. }
    ));

    let verdict = compare(&narrower).expect("突合");
    let CodekbScopeDiffView::Narrower {
        discarded_paths,
        discarded_components,
        ..
    } = verdict
    else {
        panic!("NARROWER のはず: {verdict:?}");
    };
    assert_eq!(discarded_paths, paths(&["src/auth/"]));
    assert_eq!(discarded_components, paths(&["auth-service"]));

    // 名指した相手が無いのは**判定ではなく拒否**である。
    assert_eq!(
        compare(&root.path().join("absent.md")).expect_err("拒否"),
        CompareCodekbScopeError::IncomingMissing
    );
}

/// ストアが全体走査で取込側が部分走査なら、`./` の主張そのものが失われる。
#[test]
fn a_full_scope_store_reports_a_downgrade_against_a_partial_incoming() {
    let root = tempfile::tempdir().expect("一時ディレクトリ");
    let store = write(
        root.path(),
        "store.md",
        &timestamp(
            "scope_version: 1\nkind: full\nintent: store-intent\nfingerprint: aaaa\nanalyzed:\n  paths:\n    - ./\n  components:\n    - everything",
        ),
    );
    let incoming = write(
        root.path(),
        "incoming.md",
        &timestamp(
            "scope_version: 1\nkind: partial\nintent: incoming\nanalyzed:\n  paths:\n    - src/payments/",
        ),
    );

    let verdict = CompareCodekbScopeUseCase::new(
        CodekbScopeDaoImpl::new(&store),
        CodekbScopeDaoImpl::new(&incoming),
    )
    .execute()
    .expect("突合");

    let CodekbScopeDiffView::Narrower {
        discarded_paths, ..
    } = verdict
    else {
        panic!("NARROWER のはず: {verdict:?}");
    };
    assert_eq!(discarded_paths, paths(&["./"]));
}

/// ストア側の短絡は突合モードでも先に返る（相手の不在より前）。
#[test]
fn the_store_short_circuits_before_the_incoming_file_is_looked_at() {
    let root = tempfile::tempdir().expect("一時ディレクトリ");
    let verdict = CompareCodekbScopeUseCase::new(
        CodekbScopeDaoImpl::new(&root.path().join("no-store.md")),
        CodekbScopeDaoImpl::new(&root.path().join("also-absent.md")),
    )
    .execute()
    .expect("突合");

    assert_eq!(verdict, CodekbScopeDiffView::NoStore);
}
