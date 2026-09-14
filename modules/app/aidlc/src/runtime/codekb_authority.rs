//! 群 B/C — 依頼原文の正本と codekb の読取動詞の配線
//! (`project-description` / `codekb-path` / `codekb-scope-diff`)。
//!
//! いずれも**読取専用**なので、状態ファイル・サイドカー・codekb ストア・作業ツリーという
//! リードモデルをクエリ側の DAO で読み、クエリユースケースが答えを組み、ここ
//! (合成ルート = プレゼンタ) が plain / JSON へ描く (`coding-rules/cqrs-boundaries.md`
//! 規則 6/7)。判断・導出はユースケース側にあり、ここは View の変種を選んで綴るだけである。
//!
//! # 出口の 2 層
//!
//! 読取に成功したら **stdout・exit 0** ([`Completion::emitted`] — 末尾改行は `main.rs` の
//! `writeln!` が付すので payload には含めない)。
//!
//! `codekb-scope-diff` は**判定 (verdict) を返す経路では常に exit 0** である — ライフサイクル
//! 動詞ではないので拒否を返さない (upstream の逐語コメント: "Always exits 0 with the verdict
//! in the output ... refusals are for lifecycle verbs")。exit 1 になるのは呼び出しそのものが
//! 成立しない 2 形 (`--mint` に `--paths` が無い / `--compare` の指す先が無い) だけである。
//!
//! `codekb-path` は**副作用ゼロ**である — ディレクトリを作らず、状態も監査も書かない。

use std::path::{Path, PathBuf};

use core_infrastructure::canon_json::{JsonValue, ObjectMembers, SerializationProfile, serialize};
use core_query_interface_adapter::{
    CodekbScopeDaoImpl, CodekbSourceFingerprintDaoImpl, IntentReposDaoImpl,
    ProjectDescriptionDaoImpl, StateFileDaoImpl,
};
use core_query_use_case::orchestration::{
    CodekbScopeDiffView, CompareCodekbScopeError, CompareCodekbScopeUseCase,
    DiffCodekbScopeUseCase, FindProjectDescriptionUseCase, MintCodekbFingerprintUseCase,
    ProjectDescriptionError, ProjectDescriptionView, ResolveCodekbRepoUseCase,
};

use crate::cli::CodekbArgs;
use crate::layout::Layout;
use crate::wording;

use super::Completion;

/// 依頼原文サイドカーのファイル名 (upstream `PROJECT_DESCRIPTION_FILE` 逐語)。
const PROJECT_DESCRIPTION_FILE: &str = "project-description.json";

/// codekb ストアの鮮度印のファイル名 (upstream 逐語)。
const RE_TIMESTAMP_FILE: &str = "reverse-engineering-timestamp.md";

/// 指紋が計算できないときの綴り (upstream 逐語)。
const UNKNOWN_FINGERPRINT: &str = "unknown";

/// intent が記録されていないときの綴り (upstream `store.intent || "unrecorded"`)。
const UNRECORDED_INTENT: &str = "unrecorded";

/// 単一リポジトリ構成で指紋から除く枠組みのディレクトリ (upstream 逐語)。
const WORKSPACE_FRAMEWORK_DIR: &str = "aidlc";

// ---------------------------------------------------------------------------
// 群 B — project-description
// ---------------------------------------------------------------------------

/// `aidlc-utility project-description` — 依頼原文の正本を JSON 1 行で出す。
pub(super) fn project_description(layout: &Layout) -> Completion {
    let use_case = FindProjectDescriptionUseCase::new(
        StateFileDaoImpl::new(&state_file_probe(layout)),
        ProjectDescriptionDaoImpl::new(&sidecar_probe(layout)),
    );
    match use_case.execute() {
        Ok(view) => Completion::emitted(render_description(&view)),
        Err(error) => Completion::refused(describe_description_failure(&error)),
    }
}

// ---------------------------------------------------------------------------
// 群 C — codekb-path / codekb-scope-diff
// ---------------------------------------------------------------------------

/// `aidlc-utility codekb-path` — codekb の保管先を出す (副作用なし)。
pub(super) fn codekb_path(layout: &Layout, flags: &CodekbArgs) -> Completion {
    let repo = resolve_repo(layout, flags);
    let dir = layout.relative_codekb_dir(&repo);
    if !flags.is_json() {
        // plain 形は末尾に `/` を付ける (upstream `${dir}/`)。
        return Completion::emitted(format!("{dir}/"));
    }
    let mut object = ObjectMembers::new();
    object.insert("space", JsonValue::String(layout.space().to_string()));
    object.insert("repo", JsonValue::String(repo));
    // JSON 形の `dir` には末尾の `/` を付けない (upstream の差をそのまま写す)。
    object.insert("dir", JsonValue::String(dir));
    Completion::emitted(compact(object))
}

/// `aidlc-utility codekb-scope-diff` — 走査範囲の突合 (status / `--compare` / `--mint`)。
pub(super) fn codekb_scope_diff(layout: &Layout, flags: &CodekbArgs) -> Completion {
    let repo = resolve_repo(layout, flags);
    let repo_dir = repo_source_dir(layout, &repo);
    // 単一リポジトリ構成では枠組み自身の `aidlc/` を除く — 走査記録・codekb・監査・状態を
    // 書くこと自体が全体指紋を古くしてしまわないようにするためである。
    let excluded = if repo_dir == layout.project_dir() {
        vec![WORKSPACE_FRAMEWORK_DIR.to_string()]
    } else {
        Vec::new()
    };
    let fingerprint_dao = CodekbSourceFingerprintDaoImpl::new(&repo_dir, excluded);

    if flags.is_mint() {
        return mint(&repo, flags, fingerprint_dao);
    }

    let store_dir = layout.relative_codekb_dir(&repo);
    let store_dao = CodekbScopeDaoImpl::new(&layout.codekb_dir(&repo).join(RE_TIMESTAMP_FILE));
    let outcome = match flags.compare() {
        Some(incoming) => {
            CompareCodekbScopeUseCase::new(store_dao, CodekbScopeDaoImpl::new(Path::new(incoming)))
                .execute()
                .map_err(|error| match error {
                    // 突合相手が無いのは判定ではなく拒否である (upstream もそこだけ `die()` する)。
                    CompareCodekbScopeError::IncomingMissing => {
                        wording::codekb_scope_diff_compare_not_found(incoming)
                    }
                    CompareCodekbScopeError::Unreadable(cause) => {
                        wording::codekb_scope_diff_unreadable(&cause.to_string())
                    }
                })
        }
        None => DiffCodekbScopeUseCase::new(store_dao, fingerprint_dao)
            .execute()
            .map_err(|error| wording::codekb_scope_diff_unreadable(&error.to_string())),
    };
    match outcome {
        Ok(view) => Completion::emitted(render_diff(&repo, &store_dir, &view, flags.is_json())),
        Err(diagnostic) => Completion::refused(diagnostic),
    }
}

/// `--mint` — 走査範囲の内容指紋を鋳造して出す。
fn mint(
    repo: &str,
    flags: &CodekbArgs,
    fingerprint_dao: CodekbSourceFingerprintDaoImpl,
) -> Completion {
    let paths = flags.path_list();
    if paths.is_empty() {
        return Completion::refused(wording::CODEKB_SCOPE_DIFF_MINT_REQUIRES_PATHS.to_string());
    }
    let minted = MintCodekbFingerprintUseCase::new(fingerprint_dao)
        .execute(&paths)
        .unwrap_or_else(|| UNKNOWN_FINGERPRINT.to_string());
    if !flags.is_json() {
        return Completion::emitted(minted);
    }
    let mut object = ObjectMembers::new();
    object.insert("repo", JsonValue::String(repo.to_string()));
    object.insert("fingerprint", JsonValue::String(minted));
    object.insert("paths", strings(&paths));
    Completion::emitted(compact(object))
}

/// codekb を鍵付けるリポジトリ名を決める。
pub(super) fn resolve_repo(layout: &Layout, flags: &CodekbArgs) -> String {
    let registry = layout.intents_dir().join("intents.json");
    ResolveCodekbRepoUseCase::new(IntentReposDaoImpl::new(&registry)).execute(
        flags.repo(),
        record_dir_name(layout).as_deref(),
        &workspace_name(layout),
    )
}

/// 指紋を取る対象リポジトリの根。
///
/// 兄弟ディレクトリ `<workspace>/<repo>/` が在ればそれ (多リポジトリ構成)、無ければワーク
/// スペース根そのもの (単一リポジトリ構成 — `codekbRepoName` がワークスペース名を返す場合)。
pub(super) fn repo_source_dir(layout: &Layout, repo: &str) -> PathBuf {
    let sibling = layout.project_dir().join(repo);
    if sibling.is_dir() {
        sibling
    } else {
        layout.project_dir().to_path_buf()
    }
}

/// 記録ディレクトリの名前 (record がまだ無ければ `None`)。
fn record_dir_name(layout: &Layout) -> Option<String> {
    layout
        .record_dir()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .map(str::to_string)
}

/// ワークスペースのディレクトリ名 (upstream `basename(projectDir)`)。
fn workspace_name(layout: &Layout) -> String {
    layout
        .project_dir()
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string()
}

/// 状態ファイルの所在 (record がまだ無ければ存在しないパスを渡す — `find` は不在を `None` で
/// 返し、ユースケースがそれを拒否に変える)。
fn state_file_probe(layout: &Layout) -> PathBuf {
    layout
        .state_file()
        .unwrap_or_else(|| layout.project_dir().join(".aidlc-absent-state"))
}

/// 依頼原文サイドカーの所在 (record 直下 — upstream の `join(recordRoot, …)`)。
fn sidecar_probe(layout: &Layout) -> PathBuf {
    layout.record_dir().map_or_else(
        || layout.project_dir().join(PROJECT_DESCRIPTION_FILE),
        |record| record.join(PROJECT_DESCRIPTION_FILE),
    )
}

// ---------------------------------------------------------------------------
// 描画
// ---------------------------------------------------------------------------

/// 契約 JSON の直列化は canon-json の 1 経路に固定されている (BR1.7 / ADR 0001 決定 5)。
/// `ContractCompact` は空白なし・挿入順で、upstream の `JSON.stringify` とバイト一致する。
pub(super) fn compact(object: ObjectMembers) -> String {
    serialize(
        &JsonValue::Object(object),
        SerializationProfile::ContractCompact,
    )
}

/// 文字列の配列。
pub(super) fn strings(values: &[String]) -> JsonValue {
    JsonValue::Array(
        values
            .iter()
            .map(|value| JsonValue::String(value.clone()))
            .collect(),
    )
}

/// `project-description` の JSON (列順 `description,source` を逐語で守る)。
fn render_description(view: &ProjectDescriptionView) -> String {
    let mut object = ObjectMembers::new();
    object.insert(
        "description",
        JsonValue::String(view.description().to_string()),
    );
    object.insert("source", JsonValue::String(view.source().to_string()));
    compact(object)
}

/// 判定を plain か JSON で描く。
///
/// JSON は `repo` と `store` を**先頭に**置いてから判定ごとの列を並べる (upstream の
/// `emit` が `{ repo, store, ...payload }` を組むのと同じ挿入順)。
fn render_diff(repo: &str, store_dir: &str, view: &CodekbScopeDiffView, as_json: bool) -> String {
    if !as_json {
        return render_diff_human(store_dir, view);
    }
    let mut object = ObjectMembers::new();
    object.insert("repo", JsonValue::String(repo.to_string()));
    object.insert("store", JsonValue::String(format!("{store_dir}/")));
    fill_diff_payload(&mut object, view);
    compact(object)
}

/// 判定ごとの JSON の列 (綴りと順序は upstream 逐語)。
fn fill_diff_payload(object: &mut ObjectMembers, view: &CodekbScopeDiffView) {
    let verdict = |object: &mut ObjectMembers, value: &str| {
        object.insert("verdict", JsonValue::String(value.to_string()));
    };
    match view {
        CodekbScopeDiffView::NoStore => verdict(object, "NO_STORE"),
        CodekbScopeDiffView::StoreScopeAbsent { detail } => {
            unknown_scope_payload(object, "absent", detail);
        }
        CodekbScopeDiffView::StoreScopeMalformed { detail } => {
            unknown_scope_payload(object, "malformed", detail);
        }
        CodekbScopeDiffView::IncomingScopeAbsent { detail } => {
            unknown_scope_payload(object, "absent", &format!("incoming: {detail}"));
        }
        CodekbScopeDiffView::IncomingScopeMalformed { detail } => {
            unknown_scope_payload(object, "malformed", &format!("incoming: {detail}"));
        }
        CodekbScopeDiffView::UnverifiedWithoutFingerprint {
            store_intent,
            kind,
            analyzed_paths,
        } => {
            unverified_payload(
                object,
                store_intent,
                kind,
                analyzed_paths,
                "store has no fingerprint",
            );
        }
        CodekbScopeDiffView::UnverifiedNotComputable {
            store_intent,
            kind,
            analyzed_paths,
        } => {
            unverified_payload(
                object,
                store_intent,
                kind,
                analyzed_paths,
                "fingerprint not computable here",
            );
        }
        CodekbScopeDiffView::Current {
            store_intent,
            kind,
            analyzed_paths,
            store_fingerprint,
            current_fingerprint,
        } => freshness_payload(
            object,
            "CURRENT",
            store_intent,
            kind,
            analyzed_paths,
            store_fingerprint,
            current_fingerprint,
        ),
        CodekbScopeDiffView::Stale {
            store_intent,
            kind,
            analyzed_paths,
            store_fingerprint,
            current_fingerprint,
        } => freshness_payload(
            object,
            "STALE",
            store_intent,
            kind,
            analyzed_paths,
            store_fingerprint,
            current_fingerprint,
        ),
        CodekbScopeDiffView::Covers {
            store_intent,
            incoming_intent,
        } => coverage_payload(object, "COVERS", store_intent, incoming_intent, &[], &[]),
        CodekbScopeDiffView::Narrower {
            store_intent,
            incoming_intent,
            discarded_paths,
            discarded_components,
        } => coverage_payload(
            object,
            "NARROWER",
            store_intent,
            incoming_intent,
            discarded_paths,
            discarded_components,
        ),
    }
}

fn unknown_scope_payload(object: &mut ObjectMembers, reason: &str, detail: &str) {
    object.insert("verdict", JsonValue::String("UNKNOWN_SCOPE".to_string()));
    object.insert("reason", JsonValue::String(reason.to_string()));
    object.insert("detail", JsonValue::String(detail.to_string()));
}

fn unverified_payload(
    object: &mut ObjectMembers,
    store_intent: &str,
    kind: &str,
    analyzed_paths: &[String],
    detail: &str,
) {
    object.insert("verdict", JsonValue::String("UNVERIFIED".to_string()));
    object.insert("store_intent", JsonValue::String(store_intent.to_string()));
    object.insert("kind", JsonValue::String(kind.to_string()));
    object.insert("analyzed_paths", strings(analyzed_paths));
    object.insert("detail", JsonValue::String(detail.to_string()));
}

fn freshness_payload(
    object: &mut ObjectMembers,
    verdict: &str,
    store_intent: &str,
    kind: &str,
    analyzed_paths: &[String],
    store_fingerprint: &str,
    current_fingerprint: &str,
) {
    object.insert("verdict", JsonValue::String(verdict.to_string()));
    object.insert("store_intent", JsonValue::String(store_intent.to_string()));
    object.insert("kind", JsonValue::String(kind.to_string()));
    object.insert("analyzed_paths", strings(analyzed_paths));
    object.insert(
        "store_fingerprint",
        JsonValue::String(store_fingerprint.to_string()),
    );
    object.insert(
        "current_fingerprint",
        JsonValue::String(current_fingerprint.to_string()),
    );
}

fn coverage_payload(
    object: &mut ObjectMembers,
    verdict: &str,
    store_intent: &str,
    incoming_intent: &str,
    discarded_paths: &[String],
    discarded_components: &[String],
) {
    object.insert("verdict", JsonValue::String(verdict.to_string()));
    object.insert("store_intent", JsonValue::String(store_intent.to_string()));
    object.insert(
        "incoming_intent",
        JsonValue::String(incoming_intent.to_string()),
    );
    object.insert("discarded_paths", strings(discarded_paths));
    object.insert("discarded_components", strings(discarded_components));
}

/// 判定の plain 形 (逐語は upstream のまま)。
fn render_diff_human(store_dir: &str, view: &CodekbScopeDiffView) -> String {
    match view {
        CodekbScopeDiffView::NoStore => format!(
            "NO_STORE: no reverse-engineering-timestamp.md at {store_dir}/ - first scan, nothing to compare."
        ),
        CodekbScopeDiffView::StoreScopeAbsent { detail } => store_unknown_scope("absent", detail),
        CodekbScopeDiffView::StoreScopeMalformed { detail } => {
            store_unknown_scope("malformed", detail)
        }
        CodekbScopeDiffView::IncomingScopeAbsent { detail } => {
            format!("UNKNOWN_SCOPE (incoming absent): {detail}.")
        }
        CodekbScopeDiffView::IncomingScopeMalformed { detail } => {
            format!("UNKNOWN_SCOPE (incoming malformed): {detail}.")
        }
        CodekbScopeDiffView::UnverifiedWithoutFingerprint {
            store_intent,
            analyzed_paths,
            ..
        } => unverified_human(store_intent, analyzed_paths, "recorded no fingerprint"),
        CodekbScopeDiffView::UnverifiedNotComputable {
            store_intent,
            analyzed_paths,
            ..
        } => unverified_human(
            store_intent,
            analyzed_paths,
            "the current tree's fingerprint cannot be computed",
        ),
        CodekbScopeDiffView::Current {
            store_intent,
            kind,
            analyzed_paths,
            ..
        } => format!(
            "CURRENT: the analyzed paths are unchanged since the store was built (intent: {}, coverage: {kind}):\n{}",
            recorded(store_intent),
            scope_lines(analyzed_paths)
        ),
        CodekbScopeDiffView::Stale {
            store_intent,
            analyzed_paths,
            ..
        } => format!(
            "STALE: the analyzed paths have changed since the store was built (intent: {}):\n{}",
            recorded(store_intent),
            scope_lines(analyzed_paths)
        ),
        CodekbScopeDiffView::Covers { .. } => {
            "COVERS: the incoming scan covers everything the store analyzed.".to_string()
        }
        CodekbScopeDiffView::Narrower {
            store_intent,
            incoming_intent,
            discarded_paths,
            discarded_components,
        } => {
            let components = if discarded_components.is_empty() {
                String::new()
            } else {
                format!("\n  components: {}", discarded_components.join(", "))
            };
            format!(
                "NARROWER: the incoming scope no longer claims verified deep coverage for:\n{}{components}\n(store intent: {}; incoming intent: {})",
                scope_lines(discarded_paths),
                recorded(store_intent),
                recorded(incoming_intent)
            )
        }
    }
}

fn store_unknown_scope(reason: &str, detail: &str) -> String {
    format!(
        "UNKNOWN_SCOPE ({reason}): {detail}. The store predates scope tracking. A focused merge may retain its prose, but prior paths and components are not claimed as verified coverage until rescanned."
    )
}

fn unverified_human(store_intent: &str, analyzed_paths: &[String], because: &str) -> String {
    format!(
        "UNVERIFIED: the store (intent: {}) analyzed:\n{}\nbut {because} - freshness unknown.",
        recorded(store_intent),
        scope_lines(analyzed_paths)
    )
}

/// パス一覧の箇条書き (upstream `paths.map(p => `  - ${p}`).join("\n")`)。
fn scope_lines(paths: &[String]) -> String {
    paths
        .iter()
        .map(|path| format!("  - {path}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// intent が空なら `unrecorded` (upstream `store.intent || "unrecorded"`)。
const fn recorded(intent: &str) -> &str {
    if intent.is_empty() {
        UNRECORDED_INTENT
    } else {
        intent
    }
}

/// 依頼原文を解決できなかったときの逐語 (材料はユースケース、文言はここ)。
fn describe_description_failure(error: &ProjectDescriptionError) -> String {
    match error {
        ProjectDescriptionError::StateFileAbsent => {
            wording::project_description_state_unreadable("no workflow state file")
        }
        ProjectDescriptionError::StateFileUnreadable(cause) => {
            wording::project_description_state_unreadable(&cause.to_string())
        }
        ProjectDescriptionError::MissingProjectField => {
            wording::PROJECT_DESCRIPTION_MISSING_FIELD.to_string()
        }
        ProjectDescriptionError::UnsupportedSource(source) => {
            wording::project_description_unsupported_source(source)
        }
        ProjectDescriptionError::SidecarMissing => {
            wording::PROJECT_DESCRIPTION_SIDECAR_MISSING.to_string()
        }
        // upstream は読取・復号の失敗をまとめて `failed to read …: <cause>` に包む。
        ProjectDescriptionError::SidecarUnreadable(cause) => {
            wording::project_description_sidecar_failed(&cause.to_string())
        }
        ProjectDescriptionError::SidecarNotJson(cause) => {
            wording::project_description_sidecar_failed(cause)
        }
        ProjectDescriptionError::SidecarNotAString => {
            wording::project_description_sidecar_failed(wording::PROJECT_DESCRIPTION_NOT_A_STRING)
        }
    }
}
