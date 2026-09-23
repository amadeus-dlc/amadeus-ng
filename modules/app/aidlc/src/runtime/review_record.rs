//! レビューの下書き（reviewer が書く）とレビュー記録（判定が書く）の置き場を扱う入出力境界。
//!
//! upstream 2.8.2 の配置をそのまま使う（`aidlc-lib.ts` の `reviewDraftRelativePath` /
//! `reviewRecordRelativePath`）:
//!
//! ```text
//! <record>/.aidlc-reviews/<stage>/stage/<attempt>/<iteration>.review.md   下書き
//! <record>/.aidlc-reviews/<stage>/stage/<attempt>/<iteration>.json        記録
//! ```
//!
//! 依頼が下書きの枠を開き（前の下書きを消す）、判定が下書きを読んで記録へ移す。どの下書きを
//! 判定の証拠とするか、書かれたレビューが正しいかは集約が決める。ここは観測と書き出しだけを
//! 持つ。
use super::Layout;
use super::review_brief;
use chrono::{SecondsFormat, Utc};
use core_command_domain::orchestration::{ReviewCompleted, ReviewDraft, ReviewRequested};
use core_infrastructure::canon_json::{JsonValue, Number, ObjectMembers, SerializationProfile};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// `<record>` 直下のレビュー置き場（upstream `REVIEW_RECORDS_DIR`）。
const REVIEWS_DIR: &str = ".aidlc-reviews";

/// 依頼が開いた下書きの枠を空にし、reviewer へ渡すパス（ワークスペース相対）を返す。
///
/// 同じ iteration の前の配送が残した下書きは、この配送のレビューではない（upstream
/// `openReviewDraftSlot`）。
///
/// # Errors
///
/// 記録が選ばれていない、または枠の経路が symlink を通るか通常ファイルでないものを指す場合。
pub(super) fn open_slot(layout: &Layout, requested: &ReviewRequested) -> Result<String, String> {
    let record = layout.record_dir().ok_or("no active intent record")?;
    let relative = draft_relative(
        requested.stage().as_str(),
        requested.evidence().identity().attempt(),
        requested.iteration(),
    );
    let path = record.join(&relative);
    refuse_symlinks(record, &path)?;
    match fs::symlink_metadata(&path) {
        Ok(meta) if meta.is_file() => fs::remove_file(&path).map_err(|e| e.to_string())?,
        Ok(_) => return Err(format!("{relative} is not a plain file")),
        Err(e) if e.kind() == io::ErrorKind::NotFound => (),
        Err(e) => return Err(e.to_string()),
    }
    let shown = path
        .strip_prefix(layout.project_dir())
        .map_or_else(|_| path.clone(), Path::to_path_buf);
    Ok(posix(&shown))
}

/// 判定の iteration に当たる下書きを、試行ごとに観測する。
///
/// 読めない・symlink・ハードリンクの下書きは証拠にしない（観測しなかったのと同じに扱い、
/// 集約が「レビューが無い」と断る）。
pub(super) fn drafts(layout: &Layout, stage: &str, iteration: u32) -> Vec<ReviewDraft> {
    let Some(record) = layout.record_dir() else {
        return Vec::new();
    };
    let root = record.join(REVIEWS_DIR).join(stage).join("stage");
    if refuse_symlinks(record, &root).is_err() {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(&root) else {
        return Vec::new();
    };
    let mut found: Vec<ReviewDraft> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let attempt = entry.file_name().to_str()?.to_string();
            if !is_attempt_id(&attempt) {
                return None;
            }
            let path = entry.path().join(format!("{iteration}.review.md"));
            read_plain(record, &path).map(|body| ReviewDraft::new(attempt, body))
        })
        .collect();
    found.sort_by(|a, b| a.attempt().cmp(b.attempt()));
    found
}

/// 受理された判定をレビュー記録へ書き、使った下書きを消す。記録の記録相対パスを返す。
///
/// 記録の形は upstream の `ReviewRecord`（`serializeReviewRecord`）と同じ欄・同じ順である。
/// 本文は、試行に当たる下書きがあればその原文、無ければ成果物へ追記された `## Review` 節、
/// どちらも無い（再試行済みの依頼を NOT-READY で閉じた）なら空である。
///
/// # Errors
///
/// 記録が選ばれていない、所見の表が読めない、または記録を書けない場合。
pub(super) fn write(
    layout: &Layout,
    completed: &ReviewCompleted,
    drafts: &[ReviewDraft],
) -> Result<String, String> {
    let record = layout.record_dir().ok_or("no active intent record")?;
    let request = completed.evidence().request();
    let attempt = request.identity().attempt();
    let stage = completed.stage().as_str();
    let draft = drafts.iter().find(|draft| draft.attempt() == attempt);
    let body = match draft {
        Some(draft) => draft.body().to_vec(),
        None => appended_review(
            record,
            request.appendix_artifact(),
            request.appendix_offset(),
        ),
    };
    let body = String::from_utf8(body).map_err(|e| e.to_string())?;
    let findings = if body.is_empty() {
        Vec::new()
    } else {
        review_brief::record_findings(&body, request.appendix_artifact())?
    };
    let text = |value: &str| JsonValue::String(value.to_string());
    let nullable = |value: Option<&str>| value.map_or(JsonValue::Null, text);
    let mut members = ObjectMembers::new();
    members.insert("version", JsonValue::Number(Number::PosInt(1)));
    members.insert("stage", text(stage));
    members.insert("unit", JsonValue::Null);
    members.insert("workflow", JsonValue::Null);
    members.insert("attempt", text(attempt));
    members.insert(
        "iteration",
        JsonValue::Number(Number::PosInt(u64::from(completed.iteration()))),
    );
    members.insert("reviewer", text(completed.reviewer()));
    members.insert("verdict", text(completed.verdict().as_str()));
    members.insert("request_id", text(request.identity().request_id()));
    members.insert("request_challenge", nullable(request.challenge()));
    members.insert(
        "artifact_fingerprint",
        text(completed.evidence().fingerprint()),
    );
    members.insert("source_fingerprint", nullable(request.source()));
    members.insert("unit_source_fingerprint", JsonValue::Null);
    members.insert("findings", JsonValue::Array(findings));
    members.insert("body", text(&body));
    members.insert(
        "recorded_at",
        text(&Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)),
    );
    let serialized = core_infrastructure::canon_json::serialize(
        &JsonValue::Object(members),
        SerializationProfile::ContractPretty,
    );
    let relative = format!(
        "{REVIEWS_DIR}/{stage}/stage/{attempt}/{}.json",
        completed.iteration()
    );
    let path = record.join(&relative);
    refuse_symlinks(record, &path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    core_infrastructure::atomic::write_file_atomic(&path, serialized.as_bytes())
        .map_err(|e| e.to_string())?;
    if draft.is_some() {
        let used = record.join(draft_relative(stage, attempt, completed.iteration()));
        fs::remove_file(used).map_err(|e| e.to_string())?;
    }
    Ok(relative)
}

fn draft_relative(stage: &str, attempt: &str, iteration: u32) -> String {
    format!("{REVIEWS_DIR}/{stage}/stage/{attempt}/{iteration}.review.md")
}

/// 成果物へ追記された `## Review` 節（非推奨の入力経路）。読めなければ空。
fn appended_review(record: &Path, artifact: &str, offset: usize) -> Vec<u8> {
    read_plain(record, &record.join(artifact))
        .and_then(|body| body.get(offset..).map(<[u8]>::to_vec))
        .map(|appended| {
            let start = appended
                .iter()
                .position(|b| !b.is_ascii_whitespace())
                .unwrap_or(appended.len());
            appended
                .get(start..)
                .map(<[u8]>::to_vec)
                .unwrap_or_default()
        })
        .unwrap_or_default()
}

/// symlink を通らない単一リンクの通常ファイルだけを読む。
fn read_plain(record: &Path, path: &Path) -> Option<Vec<u8>> {
    refuse_symlinks(record, path).ok()?;
    let meta = fs::symlink_metadata(path).ok()?;
    if !meta.is_file() || !single_link(&meta) {
        return None;
    }
    fs::read(path).ok()
}

#[cfg(unix)]
fn single_link(meta: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    meta.nlink() == 1
}
#[cfg(not(unix))]
fn single_link(_: &fs::Metadata) -> bool {
    true
}

/// 記録ディレクトリから `path` までの途中に symlink が無いこと。
fn refuse_symlinks(record: &Path, path: &Path) -> Result<(), String> {
    let relative = path.strip_prefix(record).map_err(|e| e.to_string())?;
    let mut current = PathBuf::from(record);
    for part in relative.components() {
        if !matches!(part, std::path::Component::Normal(_)) {
            return Err("review path outside record".into());
        }
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err("symlinked review path".into());
            }
            Ok(_) => (),
            Err(e) if e.kind() == io::ErrorKind::NotFound => break,
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

fn is_attempt_id(name: &str) -> bool {
    name.len() == 16
        && name
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

fn posix(path: &Path) -> String {
    path.components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
