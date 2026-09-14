//! D2.f の観測 — health ディレクトリと、同じ記録の進行の証拠 (本家 3062–3160)。

use std::collections::BTreeSet;

use core_query_use_case::orchestration::{HeartbeatEntryView, HeartbeatView, TimestampView};

use super::super::doctor_paths::DoctorPaths;
use super::audit_ledger::AuditLedger;

/// heartbeat と進行の観測。
pub(super) fn observe(paths: &DoctorPaths) -> HeartbeatView {
    let docs_root = paths.docs_root();
    let ledger = AuditLedger::read(&docs_root);
    let state_progressed = paths
        .record_dir()
        .and_then(|record| std::fs::read_to_string(record.join("aidlc-state.md")).ok())
        .map(|content| progressed_slugs(&content))
        .unwrap_or_default();
    let stage_or_gate: Vec<_> = ledger
        .events()
        .iter()
        .filter(|event| event.event().starts_with("STAGE_") || event.event().starts_with("GATE_"))
        .collect();
    let audit_progressed: BTreeSet<&str> = stage_or_gate
        .iter()
        .filter_map(|event| event.stage())
        .collect();
    let newest_progress = stage_or_gate
        .iter()
        .filter_map(|event| parse_millis(event.timestamp()).map(|ms| (ms, event.timestamp())))
        .max_by_key(|(ms, _)| *ms)
        .map(|(ms, raw)| TimestampView::new(raw.to_string(), Some(ms)));

    let health = docs_root.join(".aidlc-hooks-health");
    let health_dir_exists = health.exists();
    let mut has_heartbeat_files = false;
    let mut entries = Vec::new();
    if let Ok(read) = std::fs::read_dir(&health) {
        let mut files: Vec<_> = read
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "last"))
            .collect();
        files.sort();
        has_heartbeat_files = !files.is_empty();
        for file in files {
            let Ok(raw) = std::fs::read_to_string(&file) else {
                continue;
            };
            let raw = raw.trim().to_string();
            let hook = file
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default();
            let millis = parse_millis(&raw);
            entries.push(HeartbeatEntryView::new(
                hook,
                TimestampView::new(raw, millis),
            ));
        }
    }
    HeartbeatView::new(
        health_dir_exists,
        has_heartbeat_files,
        entries,
        state_progressed.len().max(audit_progressed.len()),
        ledger.content().contains("**Event**: STAGE_STARTED"),
        newest_progress,
    )
}

/// `- [<mark>] <slug> — ...` のうち pending (`[ ]`) 以外の slug (本家 `parseCheckboxes`)。
fn progressed_slugs(state: &str) -> BTreeSet<String> {
    state
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("- [")?;
            let mark = rest.chars().next()?;
            if !matches!(mark, ' ' | 'x' | 'S' | 'R' | '?' | '-') {
                return None;
            }
            let rest = rest.get(1..)?.strip_prefix("] ")?;
            let slug = rest.split_whitespace().next()?;
            let after = rest.get(slug.len()..)?.trim_start();
            after.starts_with('—').then(|| (mark, slug.to_string()))
        })
        .filter(|(mark, _)| *mark != ' ')
        .map(|(_, slug)| slug)
        .collect()
}

/// ISO 8601 UTC をエポックミリ秒へ (本家 `Date.parse` が読める綴りのうち監査が書く形)。
fn parse_millis(raw: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|at| at.timestamp_millis())
}
