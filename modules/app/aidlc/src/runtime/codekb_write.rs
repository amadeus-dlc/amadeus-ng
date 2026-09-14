//! 群 D — codekb の書込動詞の配線 (`codekb-snapshot` / `codekb-publish`)。
//!
//! どちらも**書込**である。`codekb-snapshot` は中断した公開の決着をつけてから 2 つの
//! 世代を観測し、`codekb-publish` は同じ決着のあと 3 つの compare-and-swap を確かめて
//! ストアを原子的に置き換える。決着はユースケースがロック区間の先頭で「再構成 → 集約が判断
//! → `store`」の 3 手として進めるものであり、読取メソッドの内側には隠れていない
//! (`coding-rules/command-query-separation.md`)。判断は集約、媒体は Gateway、ここ
//! (合成ルート = プレゼンタ) が持つのは**外界の実測**と**描画**である。
//!
//! # ここが実測を担う理由
//!
//! 集約は外界を読まない。源の指紋 (git の作業ツリー / 木のハッシュ) も、staged の 9 成果物も、
//! 外界にしか無い。合成ルートはコマンド側とクエリ側の両方を知ってよい唯一の場所なので
//! (`coding-rules/cqrs-boundaries.md` §対象外)、走査範囲ブロックの解読にはクエリ側の
//! 既存 DAO をそのまま使い、コマンド側へは値オブジェクトだけを渡す。既存の
//! `source_baseline.rs` / `source_fingerprint.rs` と同じ流儀である。
//!
//! # 直列化
//!
//! upstream は `withCodekbLock` (監査ロック) で囲う。こちらは同じ意図の排他ファイルロックを
//! 記録ディレクトリ配下に取る (`learnings.rs` と同じ機構)。実測 → 判断 → 置換をこの区間に
//! 収めるので、並行する別プロセスの公開と噛み合わない。

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use core_command_domain::workspace::{
    CodekbArtifact, CodekbArtifactName, CodekbArtifacts, CodekbCandidate, CodekbGeneration,
    CodekbPublishRefusal, CodekbRepoId, CodekbScopePath, CodekbScopePaths, CodekbSnapshot,
    CodekbSourceFingerprint,
};
use core_command_interface_adapter::orchestration::CodekbRepositoryImpl;
use core_command_use_case::orchestration::{
    CodekbRepository as _, PublishCodekbError, PublishCodekbUseCase, SnapshotCodekbUseCase,
};
use core_infrastructure::ExclusiveFileLock;
use core_infrastructure::canon_json::{JsonValue, ObjectMembers};
use core_infrastructure::collections::FirstClassCollection as _;
use core_infrastructure::tree_hash::hash_tree;
use core_query_interface_adapter::{CodekbScopeDaoImpl, CodekbSourceFingerprintDaoImpl};
use core_query_use_case::orchestration::{
    CodekbScopeDao as _, CodekbSourceFingerprintDao as _, ReScopeParseView,
};

use crate::cli::CodekbArgs;
use crate::layout::Layout;
use crate::wording;

use super::Completion;
use super::codekb_authority::{compact, repo_source_dir, resolve_repo, strings};

/// 鮮度印のファイル名 (upstream 逐語)。
const RE_TIMESTAMP_FILE: &str = "reverse-engineering-timestamp.md";

/// 単一リポジトリ構成で指紋から除く枠組みのディレクトリ (upstream 逐語)。
const WORKSPACE_FRAMEWORK_DIR: &str = "aidlc";

/// トランザクションディレクトリを束ねる名前 (upstream 逐語)。
const TRANSACTIONS_DIR: &str = ".aidlc-codekb-transactions";

/// ロックの待ち時間。
const LOCK_WAIT: Duration = Duration::from_secs(5);

// ---------------------------------------------------------------------------
// codekb-snapshot
// ---------------------------------------------------------------------------

/// `aidlc-utility codekb-snapshot` — 走査の直前に 2 つの世代の写しを取る。
pub(super) async fn codekb_snapshot(layout: &Layout, flags: &CodekbArgs) -> Completion {
    let (repo, id) = match resolve_repo_id(layout, flags) {
        Ok(resolved) => resolved,
        Err(diagnostic) => return Completion::refused(diagnostic),
    };
    let paths = flags.unique_path_list();
    if paths.is_empty() {
        return Completion::refused(wording::codekb_requires_paths("codekb-snapshot"));
    }
    let _lock = match acquire_lock(layout, &repo) {
        Ok(lock) => lock,
        Err(diagnostic) => return Completion::refused(diagnostic),
    };
    let Some(fingerprint) = source_fingerprint(layout, &repo, &paths) else {
        return Completion::refused(wording::codekb_snapshot_cannot_fingerprint(
            &paths.join(", "),
        ));
    };
    let mut use_case = SnapshotCodekbUseCase::new(repository(layout, &repo));
    match use_case
        .execute(&id, scope_paths(&paths), fingerprint)
        .await
    {
        Ok(snapshot) => {
            Completion::emitted(render_snapshot(layout, &repo, &snapshot, flags.is_json()))
        }
        Err(error) => Completion::refused(wording::codekb_store_failure(&error.to_string())),
    }
}

/// 写しを plain か JSON で描く。
///
/// plain は **3 行**である。末尾の改行は `main.rs` の `writeln!` が付すので、ここでは
/// 最後の行に改行を付けない (付けると 1 行余る)。
fn render_snapshot(
    layout: &Layout,
    repo: &str,
    snapshot: &CodekbSnapshot,
    as_json: bool,
) -> String {
    let paths = spelled_paths(snapshot.paths());
    if !as_json {
        return format!(
            "STORE_GENERATION {}\nSOURCE_FINGERPRINT {}\nSOURCE_PATHS {}",
            snapshot.store_generation().as_str(),
            snapshot.source_fingerprint().as_str(),
            paths.join(",")
        );
    }
    let mut object = ObjectMembers::new();
    object.insert("repo", JsonValue::String(repo.to_string()));
    object.insert(
        "store",
        JsonValue::String(format!("{}/", layout.relative_codekb_dir(repo))),
    );
    object.insert("paths", strings(&paths));
    object.insert(
        "store_generation",
        JsonValue::String(snapshot.store_generation().as_str().to_string()),
    );
    object.insert(
        "source_fingerprint",
        JsonValue::String(snapshot.source_fingerprint().as_str().to_string()),
    );
    compact(object)
}

// ---------------------------------------------------------------------------
// codekb-publish
// ---------------------------------------------------------------------------

/// `aidlc-utility codekb-publish` — compare-and-swap を確かめて候補を公開する。
pub(super) async fn codekb_publish(layout: &Layout, flags: &CodekbArgs) -> Completion {
    let (repo, id) = match resolve_repo_id(layout, flags) {
        Ok(resolved) => resolved,
        Err(diagnostic) => return Completion::refused(diagnostic),
    };
    let (Some(expected_store), Some(expected_source)) =
        (flags.expect_store(), flags.expect_source())
    else {
        return Completion::refused(wording::CODEKB_PUBLISH_REQUIRES_EXPECTATIONS.to_string());
    };
    let paths = flags.unique_path_list();
    if paths.is_empty() {
        return Completion::refused(wording::codekb_requires_paths("codekb-publish"));
    }
    let candidate = match read_candidate(layout, flags.staged()) {
        Ok(candidate) => candidate,
        Err(diagnostic) => return Completion::refused(diagnostic),
    };
    // 被覆の検査はロックの外で済ませる (upstream も同じ位置で `die()` する)。
    if let Some(uncovered) = candidate.uncovered_by(&scope_paths(&paths)) {
        return Completion::refused(wording::codekb_publish_scope_not_covered(
            uncovered.as_str(),
        ));
    }
    let (Ok(expected_store), Ok(expected_source)) = (
        CodekbGeneration::of_token(expected_store),
        CodekbSourceFingerprint::of_token(expected_source),
    ) else {
        return Completion::refused(wording::CODEKB_PUBLISH_REQUIRES_EXPECTATIONS.to_string());
    };

    let _lock = match acquire_lock(layout, &repo) {
        Ok(lock) => lock,
        Err(diagnostic) => return Completion::refused(diagnostic),
    };
    let current_source = source_fingerprint(layout, &repo, &paths);
    let current_candidate =
        bare_fingerprint(layout, &repo, &spelled_paths(candidate.analyzed_paths()));

    let mut use_case = PublishCodekbUseCase::new(repository(layout, &repo));
    if let Err(error) = use_case
        .execute(
            &id,
            candidate,
            &expected_store,
            &expected_source,
            current_source.as_ref(),
            current_candidate.as_deref(),
        )
        .await
    {
        return Completion::refused(describe_publish_failure(&error));
    }
    // 公開後の世代は**ディスクのバイトから観測する**値なので、置き換えたあとに引き直す。
    // ここは純粋な読取である — 直前の `store` が痕跡を片付けているので、畳むべき中断した
    // 公開は残っていない。
    match repository(layout, &repo).find_by_id(&id).await {
        Ok(published) => {
            let generation =
                published.take_snapshot(CodekbScopePaths::of(Vec::new()), expected_source);
            Completion::emitted(render_published(
                layout,
                &repo,
                generation.store_generation(),
                flags.is_json(),
            ))
        }
        Err(error) => Completion::refused(wording::codekb_store_failure(&error.to_string())),
    }
}

/// 公開の結果を plain か JSON で描く。
fn render_published(
    layout: &Layout,
    repo: &str,
    generation: &CodekbGeneration,
    as_json: bool,
) -> String {
    let published = format!("{}/", layout.relative_codekb_dir(repo));
    if !as_json {
        return format!("PUBLISHED {published} {}", generation.as_str());
    }
    let mut object = ObjectMembers::new();
    object.insert("repo", JsonValue::String(repo.to_string()));
    object.insert("published", JsonValue::String(published));
    object.insert(
        "generation",
        JsonValue::String(generation.as_str().to_string()),
    );
    compact(object)
}

/// 公開が通らなかったときの逐語 (材料はドメイン、文言はここ)。
fn describe_publish_failure(error: &PublishCodekbError) -> String {
    match error {
        PublishCodekbError::Refused(CodekbPublishRefusal::StoreChanged { expected, found }) => {
            wording::codekb_store_changed(expected.as_str(), found.as_str())
        }
        PublishCodekbError::Refused(CodekbPublishRefusal::SourceChanged { expected, found }) => {
            wording::codekb_source_changed(
                expected.as_str(),
                found.as_ref().map(CodekbSourceFingerprint::as_str),
            )
        }
        PublishCodekbError::Refused(CodekbPublishRefusal::CandidateStale { staged, current }) => {
            wording::codekb_candidate_stale(staged.as_deref(), current.as_deref())
        }
        PublishCodekbError::Repository(failure) => {
            wording::codekb_store_failure(&failure.to_string())
        }
    }
}

// ---------------------------------------------------------------------------
// staged の公開候補を読む (upstream `readCodekbCandidate`)
// ---------------------------------------------------------------------------

/// staged ディレクトリを検査して公開候補を組む。
///
/// 検査の順序も逐語である — 指定の有無 → 封じ込め → 在否 → 種別 → リンクされた親 →
/// 9 成果物ちょうど → 通常ファイル → 走査範囲ブロック。
fn read_candidate(layout: &Layout, staged: Option<&str>) -> Result<CodekbCandidate, String> {
    let Some(staged) = staged else {
        return Err(wording::CODEKB_PUBLISH_REQUIRES_STAGED.to_string());
    };
    let project_dir = layout.project_dir();
    let path = if Path::new(staged).is_absolute() {
        PathBuf::from(staged)
    } else {
        project_dir.join(staged)
    };
    if !is_contained(project_dir, &path) {
        return Err(wording::CODEKB_PUBLISH_STAGED_OUTSIDE.to_string());
    }
    let Ok(metadata) = fs::symlink_metadata(&path) else {
        return Err(wording::codekb_publish_staged_not_found(staged));
    };
    if !metadata.is_dir() {
        return Err(wording::CODEKB_PUBLISH_STAGED_NOT_A_DIRECTORY.to_string());
    }
    // リンクされた親を通って外へ出ていないか (実体で確かめる)。
    match (fs::canonicalize(project_dir), fs::canonicalize(&path)) {
        (Ok(real_project), Ok(real_staged)) => {
            if !is_contained(&real_project, &real_staged) {
                return Err(wording::CODEKB_PUBLISH_STAGED_ESCAPES.to_string());
            }
        }
        _ => return Err(wording::codekb_publish_staged_not_found(staged)),
    }

    let mut found: Vec<String> = Vec::new();
    let entries =
        fs::read_dir(&path).map_err(|error| wording::codekb_store_failure(&error.to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|error| wording::codekb_store_failure(&error.to_string()))?;
        found.push(entry.file_name().to_string_lossy().into_owned());
    }
    found.sort();
    let canonical: Vec<String> = CodekbArtifactName::all()
        .iter()
        .map(|name| name.as_str().to_string())
        .collect();
    if found != canonical {
        return Err(wording::codekb_publish_staged_not_the_nine(&found));
    }

    let mut artifacts: Vec<CodekbArtifact> = Vec::new();
    for name in CodekbArtifactName::all() {
        let file = path.join(name.as_str());
        let Ok(metadata) = fs::symlink_metadata(&file) else {
            return Err(wording::codekb_publish_staged_not_a_regular_file(
                name.as_str(),
            ));
        };
        if !metadata.is_file() {
            return Err(wording::codekb_publish_staged_not_a_regular_file(
                name.as_str(),
            ));
        }
        let bytes =
            fs::read(&file).map_err(|error| wording::codekb_store_failure(&error.to_string()))?;
        artifacts.push(CodekbArtifact::new(name, bytes));
    }
    let artifacts = CodekbArtifacts::of(artifacts)
        .map_err(|error| wording::codekb_store_failure(&error.to_string()))?;

    // 走査範囲ブロックの解読はクエリ側の既存 DAO を使う (合成ルートは両側を知ってよい)。
    let parsed = CodekbScopeDaoImpl::new(&path.join(RE_TIMESTAMP_FILE))
        .find()
        .map_err(|error| wording::codekb_store_failure(&error.to_string()))?;
    match parsed {
        Some(ReScopeParseView::Parsed(scope)) => Ok(CodekbCandidate::new(
            artifacts,
            scope_paths(scope.analyzed_paths()),
            scope.fingerprint().map(str::to_string),
        )),
        Some(ReScopeParseView::Absent(detail)) => Err(wording::codekb_publish_invalid_scope_block(
            "absent", &detail,
        )),
        Some(ReScopeParseView::Malformed(detail)) => Err(
            wording::codekb_publish_invalid_scope_block("malformed", &detail),
        ),
        None => Err(wording::codekb_publish_invalid_scope_block(
            "absent",
            "no reverse-engineering-timestamp.md",
        )),
    }
}

/// `root` の**内側**を指しているか (`..` で外へ出ていないか)。
fn is_contained(root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    !relative.as_os_str().is_empty()
        && !relative
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
}

// ---------------------------------------------------------------------------
// 実測と結線
// ---------------------------------------------------------------------------

/// リポジトリ識別子を解決し、パス片として成立することを確かめる。
fn resolve_repo_id(layout: &Layout, flags: &CodekbArgs) -> Result<(String, CodekbRepoId), String> {
    let repo = resolve_repo(layout, flags);
    CodekbRepoId::parse(&repo)
        .map(|id| (repo.clone(), id))
        .map_err(|_| wording::codekb_invalid_repo(&repo))
}

/// ストアの Gateway を結線する。
fn repository(layout: &Layout, repo: &str) -> CodekbRepositoryImpl {
    CodekbRepositoryImpl::new(
        &layout.codekb_dir(repo),
        &layout.intents_dir().join(TRANSACTIONS_DIR).join(repo),
    )
}

/// 走査範囲の指紋を実測する — git の作業ツリーが先、採れなければ木のハッシュへ後退する。
fn source_fingerprint(
    layout: &Layout,
    repo: &str,
    paths: &[String],
) -> Option<CodekbSourceFingerprint> {
    bare_fingerprint(layout, repo, paths).map(|hash| {
        if hash.starts_with("tree ") {
            CodekbSourceFingerprint::of_tree(hash.trim_start_matches("tree "))
        } else {
            CodekbSourceFingerprint::of_git(&hash)
        }
    })
}

/// 由来の印を付けない生のハッシュ (git 由来はそのまま、木のハッシュは `tree ` を前置する)。
fn bare_fingerprint(layout: &Layout, repo: &str, paths: &[String]) -> Option<String> {
    if paths.is_empty() {
        return None;
    }
    let repo_dir = repo_source_dir(layout, repo);
    let excluded = if repo_dir == layout.project_dir() {
        vec![WORKSPACE_FRAMEWORK_DIR.to_string()]
    } else {
        Vec::new()
    };
    if let Some(hash) = CodekbSourceFingerprintDaoImpl::new(&repo_dir, excluded.clone()).find(paths)
    {
        return Some(hash);
    }
    hash_tree(&repo_dir, paths, &excluded).map(|hash| format!("tree {hash}"))
}

/// 綴りの一覧を走査範囲のパスへ写す (空綴りは落とす)。
fn scope_paths(values: &[String]) -> CodekbScopePaths {
    CodekbScopePaths::of(
        values
            .iter()
            .filter_map(|value| CodekbScopePath::parse(value).ok())
            .collect(),
    )
}

/// 走査範囲のパスを綴りの一覧へ戻す。
fn spelled_paths(paths: &CodekbScopePaths) -> Vec<String> {
    paths.fold_left(Vec::new(), |mut acc, path| {
        acc.push(path.as_str().to_string());
        acc
    })
}

/// 公開を直列化する排他ロックを取る (upstream `withCodekbLock` に対応する区間)。
fn acquire_lock(layout: &Layout, repo: &str) -> Result<ExclusiveFileLock, String> {
    let path = layout
        .intents_dir()
        .join(format!(".aidlc-codekb-{repo}.lock"));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| wording::codekb_store_failure(&error.to_string()))?;
    }
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|error| wording::codekb_store_failure(&error.to_string()))?;
    ExclusiveFileLock::acquire(file, LOCK_WAIT)
        .map_err(|error| wording::codekb_store_failure(&error.to_string()))
}
