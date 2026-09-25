//! Createdに記録した公開名を共有登録へ投影する。イベント外の既存行は保持する。
use super::{JournalBatch, PublicationFile, ReadModelUpdateError};
use crate::read_tables::ReadTables;
use core_command_domain::orchestration::IntentExecutionEvent;
use std::{fs, io, path::Path};

pub(super) fn publication(
    history: &JournalBatch,
    tables: &ReadTables,
    path: &Path,
) -> Result<Option<PublicationFile>, ReadModelUpdateError> {
    if !history
        .intents()
        .iter()
        .any(|intent| intent.record_name().is_some())
    {
        return Ok(None);
    }
    let failure = |kind| ReadModelUpdateError::PublicationIo {
        path: path.to_path_buf(),
        kind,
    };
    let before = match fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(failure(error.kind())),
    };
    let mut rows: Vec<serde_json::Value> = match &before {
        Some(text) => {
            serde_json::from_str(text).map_err(|_| failure(io::ErrorKind::InvalidData))?
        }
        None => Vec::new(),
    };
    let mut changed = false;
    for intent in history.intents() {
        let Some(name) = intent.record_name() else {
            continue;
        };
        let execution = history
            .executions()
            .iter()
            .rev()
            .find_map(|entry| match entry.event() {
                IntentExecutionEvent::Started(started) if started.intent_id() == intent.id() => {
                    tables
                        .executions()
                        .iter()
                        .find(|row| row.id() == started.aggregate_id().as_str())
                }
                _ => None,
            });
        // 開始の事実がまだなければ、登録を先行して公開しない。
        let Some(execution) = execution else { continue };
        let status = if execution.status() == "completed" {
            "complete"
        } else {
            "in-flight"
        };
        let proposed = serde_json::Value::Object(
            [
                ("uuid", intent.id().as_str()),
                ("slug", name.slug()),
                ("dirName", name.directory().as_str()),
                ("scope", intent.scope()),
                ("status", status),
            ]
            .into_iter()
            .map(|(key, value)| {
                (
                    key.to_string(),
                    serde_json::Value::String(value.to_string()),
                )
            })
            .collect(),
        );
        if let Some(existing) = rows.iter_mut().find(|row| {
            row.get("uuid").and_then(serde_json::Value::as_str) == Some(intent.id().as_str())
        }) {
            if existing.get("dirName").and_then(serde_json::Value::as_str)
                != Some(name.directory().as_str())
            {
                return Err(ReadModelUpdateError::PublicationConflict {
                    path: path.to_path_buf(),
                });
            }
            let object = existing
                .as_object_mut()
                .ok_or_else(|| failure(io::ErrorKind::InvalidData))?;
            if object.get("status").and_then(serde_json::Value::as_str) != Some(status) {
                object.insert(
                    "status".to_string(),
                    serde_json::Value::String(status.to_string()),
                );
                changed = true;
            }
        } else {
            if rows.iter().any(|row| {
                row.get("dirName").and_then(serde_json::Value::as_str)
                    == Some(name.directory().as_str())
            }) {
                return Err(ReadModelUpdateError::PublicationConflict {
                    path: path.to_path_buf(),
                });
            }
            rows.push(proposed);
            changed = true;
        }
    }
    if !changed {
        return Ok(None);
    }
    let value = core_infrastructure::canon_json::to_value(&rows)
        .map_err(|_| failure(io::ErrorKind::InvalidData))?;
    let after = core_infrastructure::canon_json::serialize(
        &value,
        core_infrastructure::canon_json::SerializationProfile::ContractPretty,
    );
    Ok(Some(match before {
        Some(before) => PublicationFile::replacement(path, &before, &after),
        None => PublicationFile::creation(path, &after),
    }))
}
