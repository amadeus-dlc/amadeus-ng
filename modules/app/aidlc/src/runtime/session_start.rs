//! SessionStartのnavigation、監査、純粋な文面への接続。
use super::{
    Completion, Layout,
    session_hooks::{field, record},
};
use crate::{intent_location::IntentLocation, session_navigation::SessionNavigation};
use core_command_domain::workspace::{AuditFieldKey, AuditFields, EventType};
use harness_claude::{SessionContextNotices, SessionStartContext, SessionWorkflowFields};

pub(super) async fn run(layout: &Layout, input: &str) -> Completion {
    let envelope = harness_claude::SessionStartEnvelope::parse(input);
    let session = envelope
        .session()
        .filter(|session| Layout::valid_session_id(session))
        .unwrap_or_default();
    let navigation = SessionNavigation::new(layout.project_dir(), session);
    SessionNavigation::write_transcript(layout.project_dir(), session, envelope.transcript());
    if let Some(navigation) = &navigation {
        let _ = navigation.write_current();
        navigation.write_ancestry();
    }
    let binding = navigation.as_ref().and_then(SessionNavigation::binding);
    let stamp = navigation.as_ref().and_then(SessionNavigation::stamp);
    if let Some(navigation) = &navigation
        && (["startup", "clear"].contains(&envelope.source())
            || binding.is_none() && navigation.offer().is_some())
    {
        let _ = navigation.clear_offer();
    }
    let stamped_target = (envelope.source() == "resume" && binding.is_none())
        .then(|| {
            stamp
                .as_deref()
                .and_then(|uuid| IntentLocation::find(layout.project_dir(), uuid))
        })
        .flatten();
    let selected = stamped_target.as_ref().map_or_else(
        || Layout::resolve_for_session(layout.project_dir(), Some(session)),
        |found| found.layout().clone(),
    );
    if let Some(navigation) = &navigation {
        let _ = navigation.write_binding(&selected);
    }
    SessionNavigation::bootstrap(&selected);
    let Some(state_path) = selected.state_file().filter(|path| path.exists()) else {
        return if session.is_empty() {
            Completion::silent()
        } else {
            Completion::emitted(SessionStartContext::runtime(session).to_json_line())
        };
    };
    let heartbeat = super::observe_hook_health(&selected, "session-start").await;
    if heartbeat.code != 0 {
        return heartbeat;
    }
    let kind = if envelope.rebind_check() {
        None
    } else {
        match envelope.source() {
            "startup" | "clear" | "malformed" => Some(EventType::SessionStarted),
            "resume" => Some(EventType::SessionResumed),
            _ => None,
        }
    };
    if let Some(kind) = kind {
        let source_key = match AuditFieldKey::parse("Source") {
            Ok(key) => key,
            Err(error) => return Completion::refused(error.to_string()),
        };
        let mut fields = AuditFields::new().with(source_key, envelope.source());
        if !session.is_empty()
            && let Ok(key) = AuditFieldKey::parse("Session")
        {
            fields = fields.with(key, session);
        }
        if let Err(error) = record(&selected, kind, fields, String::new()).await {
            let _ = super::record_hook_drop(&selected, "session-start", &error).await;
        }
    }
    let shared = Layout::shared(layout.project_dir());
    let live = IntentLocation::current(&shared);
    let selected_intent = IntentLocation::current(&selected);
    let mut rebind_offer = String::new();
    if let Some(navigation) = &navigation {
        if kind == Some(EventType::SessionStarted) {
            if let Some(intent) = &selected_intent {
                let _ = navigation.write_stamp(intent.uuid());
            }
        } else if envelope.source() == "resume" {
            let owned = if binding.is_some() {
                selected_intent.as_ref().map(|intent| intent.uuid())
            } else {
                stamp.as_deref()
            };
            if let Some(owned) =
                owned.filter(|owned| Some(*owned) != live.as_ref().map(|intent| intent.uuid()))
            {
                if let Some(was) = IntentLocation::find(layout.project_dir(), owned) {
                    let from = was
                        .layout()
                        .record_dir()
                        .and_then(std::path::Path::file_name)
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let to = shared
                        .record_dir()
                        .and_then(std::path::Path::file_name)
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "(none)".into());
                    let signature =
                        format!("{}/{from}->{}/{to}", was.layout().space(), shared.space());
                    if navigation.offer().as_deref() != Some(signature.as_str()) {
                        let instruction = if was.layout().space() == shared.space() {
                            format!("run `/aidlc intent {}`", was.slug())
                        } else {
                            format!(
                                "first run `/aidlc space {}`; after it completes, run `/aidlc intent {}`",
                                was.layout().space(),
                                was.slug()
                            )
                        };
                        rebind_offer = SessionStartContext::format_rebind_offer(
                            was.slug(),
                            live.as_ref().map_or("(none)", |intent| intent.slug()),
                            &instruction,
                        );
                        let _ = navigation.write_offer(&signature);
                    }
                }
            } else {
                let _ = navigation.clear_offer();
            }
            if (binding.is_some() || stamped_target.is_some()) && selected_intent.is_some() {
                if let Some(intent) = &selected_intent {
                    let _ = navigation.write_stamp(intent.uuid());
                }
            } else if binding.is_some() && stamp.is_some() {
                let _ = navigation.clear_stamp();
            } else if let Some(live) = &live {
                let _ = navigation.write_stamp(live.uuid());
            } else if stamp.is_some() {
                let _ = navigation.clear_stamp();
            }
        } else if stamp.is_none()
            && let Some(intent) = &selected_intent
        {
            let _ = navigation.write_stamp(intent.uuid());
        }
    }
    if envelope.rebind_check() {
        return if rebind_offer.is_empty() {
            Completion::silent()
        } else {
            Completion::emitted(
                SessionStartContext::rebind_probe(session, &rebind_offer).to_json_line(),
            )
        };
    }
    let bytes = match std::fs::read(state_path) {
        Ok(bytes) => bytes,
        Err(error) => return Completion::refused(error.to_string()),
    };
    let state = String::from_utf8_lossy(&bytes);
    let fields = SessionWorkflowFields::new(
        field(&state, "Scope").unwrap_or_else(|| "unknown".into()),
        field(&state, "Lifecycle Phase").unwrap_or_else(|| "unknown".into()),
        field(&state, "Current Stage").unwrap_or_else(|| "unknown".into()),
        field(&state, "Status").unwrap_or_else(|| "unknown".into()),
        field(&state, "Active Agent").unwrap_or_else(|| "unknown".into()),
        field(&state, "Last Completed Stage").unwrap_or_else(|| "none".into()),
        field(&state, "Next Action").unwrap_or_else(|| "resume current stage".into()),
    );
    let unit = field(&state, "Active Unit")
        .filter(|value| !value.is_empty())
        .map_or_else(String::new, |unit| {
            let status = field(&state, "Unit State").unwrap_or_else(|| "in-progress".into());
            let reason = field(&state, "Unit Pause Reason")
                .filter(|value| !value.is_empty())
                .map_or_else(String::new, |value| format!("; reason: {value}"));
            let next = field(&state, "Unit Next Action")
                .filter(|value| !value.is_empty())
                .map_or_else(String::new, |value| format!("; next: {value}"));
            format!("Active Unit: {unit} ({status}{reason}{next})\n")
        });
    let recovery = selected
        .record_dir()
        .is_some_and(|record| record.join(".aidlc-recovery.md").exists());
    let notices =
        SessionContextNotices::new(rebind_offer, unit, recovery, uncompiled_stages(&selected));
    Completion::emitted(SessionStartContext::workflow(&fields, session, &notices).to_json_line())
}

fn uncompiled_stages(layout: &Layout) -> Vec<String> {
    let graph = std::env::var_os("AIDLC_STAGE_GRAPH").map_or_else(
        || layout.definition_data_dir().join("stage-graph.json"),
        std::path::PathBuf::from,
    );
    let Some(graph) = std::fs::read(graph)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
    else {
        return Vec::new();
    };
    let Some(nodes) = graph.as_array() else {
        return Vec::new();
    };
    let known: std::collections::BTreeSet<_> = nodes
        .iter()
        .filter_map(|node| node.get("slug").and_then(serde_json::Value::as_str))
        .collect();
    let root = std::env::var_os("AIDLC_STAGES_DIR")
        .map_or_else(|| layout.stage_library_dir(), std::path::PathBuf::from);
    let mut unknown = std::collections::BTreeSet::new();
    for phase in [
        "initialization",
        "ideation",
        "inception",
        "construction",
        "operation",
    ] {
        if let Ok(entries) = std::fs::read_dir(root.join(phase)) {
            for entry in entries.filter_map(Result::ok) {
                if let Some(name) = entry
                    .file_name()
                    .to_str()
                    .and_then(|name| name.strip_suffix(".md"))
                    && !known.contains(name)
                {
                    unknown.insert(name.to_string());
                }
            }
        }
    }
    unknown.into_iter().collect()
}
