//! D4.a〜D5.b — 作業記録がある場合の状態・識別・ストア・投影。

use crate::workspace::{
    DoctorCheck, DoctorCheckId, ProjectionObservation, RecordObservation, StateFileObservation,
    StateVersionKind, StoreObservation,
};

const STATE_READABLE_LABEL: &str = "Native workflow state readable";
const IDENTITY_LABEL: &str = "Native workflow identity";
const STORE_LABEL: &str = "Native event store readable";
const PROJECTION_LABEL: &str = "Native projection consistency";
/// この build が読み書きする表 — 欠けていれば対応 schema ではない。
const REQUIRED_TABLES: [&str; 3] = ["amadeus_projection_checkpoint", "journal", "read_execution"];

/// D4.a (状態ファイルがある場合)、D4.b、D4.c、D5.a、D5.b。
pub(super) fn evaluate(record: &RecordObservation) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();
    let state_relative = record.location().observed().map(|observed| {
        format!(
            "{}/{observed}/aidlc-state.md",
            record.location().intents_relative()
        )
    });
    match record.state() {
        StateFileObservation::Classified(version) => {
            let message = version.message().unwrap_or_default().to_string();
            let label = match version.kind() {
                StateVersionKind::Ok => None,
                StateVersionKind::Unparseable => Some("state version readable"),
                StateVersionKind::Past => Some("state version current"),
                StateVersionKind::Future => Some("state version compatible"),
            };
            checks.push(label.map_or_else(
                || DoctorCheck::passed(DoctorCheckId::D4a, "State Version: 8".to_string()),
                |label| DoctorCheck::failed(DoctorCheckId::D4a, label.to_string(), message),
            ));
            checks.push(DoctorCheck::passed(
                DoctorCheckId::D4b,
                STATE_READABLE_LABEL.to_string(),
            ));
        }
        StateFileObservation::Unreadable(cause) => checks.push(DoctorCheck::failed(
            DoctorCheckId::D4b,
            STATE_READABLE_LABEL.to_string(),
            format!(
                "{}: unreadable ({cause})",
                state_relative.as_deref().unwrap_or("aidlc-state.md")
            ),
        )),
        StateFileObservation::Absent => checks.push(DoctorCheck::failed(
            DoctorCheckId::D4b,
            STATE_READABLE_LABEL.to_string(),
            format!(
                "{}: missing",
                state_relative.as_deref().unwrap_or("aidlc-state.md")
            ),
        )),
    }
    checks.push(match identity_cause(record) {
        None => DoctorCheck::passed(DoctorCheckId::D4c, IDENTITY_LABEL.to_string()),
        Some(cause) => DoctorCheck::failed(DoctorCheckId::D4c, IDENTITY_LABEL.to_string(), cause),
    });
    let store = record.location().store_relative();
    checks.push(match record.store() {
        StoreObservation::Absent => DoctorCheck::failed(
            DoctorCheckId::D5a,
            STORE_LABEL.to_string(),
            format!("{store}: missing"),
        ),
        StoreObservation::Unreadable(cause) => DoctorCheck::failed(
            DoctorCheckId::D5a,
            STORE_LABEL.to_string(),
            format!("{store}: unreadable ({cause})"),
        ),
        StoreObservation::Opened(schema) => {
            let missing: Vec<&str> = REQUIRED_TABLES
                .iter()
                .copied()
                .filter(|required| !schema.has_table(required))
                .collect();
            let mut problems = Vec::new();
            if !missing.is_empty() {
                problems.push(format!("missing tables: {}", missing.join(", ")));
            }
            if schema.schema_version() == 0 {
                problems.push("read schema version 0".to_string());
            }
            if problems.is_empty() {
                DoctorCheck::passed(DoctorCheckId::D5a, STORE_LABEL.to_string())
            } else {
                DoctorCheck::failed(
                    DoctorCheckId::D5a,
                    STORE_LABEL.to_string(),
                    format!("{store}: incompatible schema ({})", problems.join("; ")),
                )
            }
        }
    });
    checks.push(match projection_cause(record) {
        None => DoctorCheck::passed(DoctorCheckId::D5b, PROJECTION_LABEL.to_string()),
        Some(cause) => DoctorCheck::failed(DoctorCheckId::D5b, PROJECTION_LABEL.to_string(), cause),
    });
    checks
}

/// D4.c の失敗原因 — 選択の曖昧さ・カーソル・登録簿・実行行の intent の対応。
fn identity_cause(record: &RecordObservation) -> Option<String> {
    let location = record.location();
    let Some(selected) = location.selected() else {
        return Some(location.cursor_target().map_or_else(
            || {
                format!(
                    "{}: ambiguous ({} records, no active record selected)",
                    location.intents_relative(),
                    location.records().len()
                )
            },
            |target| {
                format!(
                    "{}/active-intent: unresolved (names {target}, which has no aidlc-state.md)",
                    location.intents_relative()
                )
            },
        ));
    };
    let cursor_relative = format!(
        "{}/{selected}/.aidlc-execution",
        location.intents_relative()
    );
    let cursor = match record.cursor() {
        Ok(Some(cursor)) => cursor,
        Ok(None) => return Some(format!("{cursor_relative}: missing")),
        Err(cause) => return Some(format!("{cursor_relative}: unreadable ({cause})")),
    };
    let registry_relative = format!("{}/intents.json", location.intents_relative());
    match record.registry_directory() {
        Ok(Some(directory)) if directory == selected => {}
        Ok(Some(directory)) => {
            return Some(format!(
                "{registry_relative}: mismatch (intent {} is registered to {directory})",
                cursor.intent_id()
            ));
        }
        Ok(None) => {
            return Some(format!(
                "{registry_relative}: missing entry for intent {}",
                cursor.intent_id()
            ));
        }
        Err(cause) => return Some(format!("{registry_relative}: unreadable ({cause})")),
    }
    match record.projection() {
        Ok(projection) => match projection.execution_intent_id() {
            Some(intent) if intent == cursor.intent_id() => None,
            Some(intent) => Some(format!(
                "{}: mismatch (execution {} belongs to intent {intent})",
                location.store_relative(),
                cursor.execution_id()
            )),
            None => Some(format!(
                "{}: projection unavailable (execution {} not projected)",
                location.store_relative(),
                cursor.execution_id()
            )),
        },
        Err(cause) => Some(format!(
            "{}: unreadable ({cause})",
            location.store_relative()
        )),
    }
}

/// D5.b の失敗原因 — 実行行・チェックポイント・未反映・未確定の公開・監査シャード。
fn projection_cause(record: &RecordObservation) -> Option<String> {
    let location = record.location();
    let store = location.store_relative();
    let projection: &ProjectionObservation = match record.projection() {
        Ok(projection) => projection,
        Err(cause) => return Some(format!("{store}: projection unavailable ({cause})")),
    };
    let execution = record
        .cursor()
        .as_ref()
        .ok()
        .and_then(|cursor| cursor.as_ref())
        .map(|cursor| cursor.execution_id().to_string())
        .unwrap_or_default();
    if projection.execution_intent_id().is_none() {
        return Some(format!(
            "{store}: projection unavailable (execution {execution} not projected)"
        ));
    }
    let Some(checkpoint) = projection.checkpoint() else {
        return Some(format!(
            "{store}: projection unavailable (no checkpoint for orchestration-{execution})"
        ));
    };
    if let Some(latest) = projection.latest_execution_event()
        && latest > checkpoint
    {
        return Some(format!(
            "{store}: projection unavailable (journal position {latest} is ahead of checkpoint {checkpoint})"
        ));
    }
    if projection.pending_publications() > 0 {
        return Some(format!(
            "{store}: projection unavailable ({} pending publications)",
            projection.pending_publications()
        ));
    }
    if projection.audit_shard_count() == 0 {
        let audit = location.selected().map_or_else(
            || format!("{}/audit", location.intents_relative()),
            |selected| format!("{}/{selected}/audit", location.intents_relative()),
        );
        return Some(format!("{audit}: projection unavailable (no audit shard)"));
    }
    None
}
