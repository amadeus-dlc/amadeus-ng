//! D4 / D5 の観測 — 作業記録がある場合の状態・カーソル・登録簿・ストア・投影。

use std::path::Path;

use core_query_use_case::orchestration::{
    ExecutionCursorView, ObservationFailure, RecordLocationView, RecordObservationView,
    StateFileObservationView, StoreObservationView,
};

use super::super::doctor_paths::DoctorPaths;
use super::super::state_version_classifier::StateVersionClassifier;
use super::audit_ledger::AuditLedger;
use super::store;

/// 記録もストアも無ければ `None` (初回状態)。
pub(super) fn observe<C: StateVersionClassifier>(
    paths: &DoctorPaths,
    classifier: &C,
) -> Option<RecordObservationView> {
    let intents = paths.intents_dir();
    let records = record_names(&intents);
    if records.is_empty() && !paths.store_path().exists() {
        return None;
    }
    let selected = paths
        .record_dir()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned());
    let cursor_target = std::fs::read_to_string(intents.join("active-intent"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|name| !name.is_empty() && records.contains(name));
    // 観測の対象: 選ばれた記録、さもなくばカーソルが名指す記録 (状態ファイルを失った記録は
    // 本家の規則で選ばれないが、その欠落を名指して報告するために読む)。
    let observed_dir = paths
        .record_dir()
        .map(Path::to_path_buf)
        .or_else(|| cursor_target.as_ref().map(|name| intents.join(name)));
    let state = observed_dir
        .as_deref()
        .map_or(StateFileObservationView::Absent, |record| {
            state_of(&record.join("aidlc-state.md"), classifier)
        });
    let cursor = observed_dir.as_deref().map_or(Ok(None), |record| {
        cursor_of(&record.join(".aidlc-execution"))
    });
    let registry_directory = match &cursor {
        Ok(Some(cursor)) => {
            registry_directory_of(&intents.join("intents.json"), cursor.intent_id())
        }
        _ => Ok(None),
    };
    let (store_view, opened) = store::observe(paths.store_path());
    let audit_shard_count = AuditLedger::read(&paths.docs_root()).shard_count();
    let projection = match (&opened, &cursor) {
        (Some(store), Ok(Some(cursor))) => {
            store::projection(store, cursor.execution_id(), audit_shard_count)
        }
        (None, _) => Err(ObservationFailure::new(match &store_view {
            StoreObservationView::Absent => "store missing".to_string(),
            StoreObservationView::Unreadable(cause) => format!("store unreadable: {cause}"),
            StoreObservationView::Opened(_) => "store unavailable".to_string(),
        })),
        (Some(_), Ok(None)) => Err(ObservationFailure::new(
            "execution cursor missing".to_string(),
        )),
        (Some(_), Err(cause)) => Err(ObservationFailure::new(format!(
            "execution cursor unreadable: {cause}"
        ))),
    };
    Some(RecordObservationView::new(
        RecordLocationView::new(
            paths.relative(&intents),
            paths.relative(paths.store_path()),
            records,
            selected,
            cursor_target,
        ),
        state,
        cursor,
        registry_directory,
        store_view,
        projection,
    ))
}

/// intents ディレクトリ直下の、`.` で始まらないディレクトリ名 (名前順)。
fn record_names(intents: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(intents)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| !name.starts_with('.'))
        .collect();
    names.sort();
    names
}

/// 状態ファイルの 3 態。分類は注入された U2 の分類器が行う。
fn state_of<C: StateVersionClassifier>(path: &Path, classifier: &C) -> StateFileObservationView {
    if !path.exists() {
        return StateFileObservationView::Absent;
    }
    match std::fs::read_to_string(path) {
        Ok(content) => StateFileObservationView::Classified(classifier.classify(&content)),
        Err(error) => {
            StateFileObservationView::Unreadable(ObservationFailure::new(error.to_string()))
        }
    }
}

/// `<record>/.aidlc-execution` (1 行目 = 実行、2 行目 = intent)。
fn cursor_of(path: &Path) -> Result<Option<ExecutionCursorView>, ObservationFailure> {
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)
        .map_err(|error| ObservationFailure::new(error.to_string()))?;
    let mut lines = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    match (lines.next(), lines.next(), lines.next()) {
        (Some(execution), Some(intent), None) if is_uuid(execution) && is_uuid(intent) => Ok(Some(
            ExecutionCursorView::new(execution.to_string(), intent.to_string()),
        )),
        _ => Err(ObservationFailure::new(
            "malformed execution cursor".to_string(),
        )),
    }
}

/// `8-4-4-4-12` の 16 進 (識別子の綴りの検査は合成ルートの型が持つ — ここは形だけ)。
fn is_uuid(value: &str) -> bool {
    let groups: Vec<&str> = value.split('-').collect();
    groups.len() == 5
        && groups
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(group, len)| group.len() == len && group.chars().all(|c| c.is_ascii_hexdigit()))
}

/// 登録簿 (`intents.json`) が intent に対応付ける記録ディレクトリ名。
fn registry_directory_of(
    path: &Path,
    intent_id: &str,
) -> Result<Option<String>, ObservationFailure> {
    let bytes = std::fs::read(path).map_err(|error| ObservationFailure::new(error.to_string()))?;
    let entries: Vec<serde_json::Value> = serde_json::from_slice(&bytes)
        .map_err(|error| ObservationFailure::new(format!("invalid JSON: {error}")))?;
    let mut found = None;
    for entry in entries {
        if entry.get("uuid").and_then(serde_json::Value::as_str) != Some(intent_id) {
            continue;
        }
        let directory = entry
            .get("dirName")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ObservationFailure::new("entry without dirName".to_string()))?;
        if found.is_some() {
            return Err(ObservationFailure::new(format!(
                "duplicate entries for intent {intent_id}"
            )));
        }
        found = Some(directory.to_string());
    }
    Ok(found)
}
