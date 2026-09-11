//! セッション補助フックから保存・通常RMU・通知ID Queryへの接続。
use super::{Completion, Layout, Utc};
use core_command_domain::workspace::{
    AuditFieldKey, AuditFields, EventType, HookHealthTarget, IntentDirName,
    SessionAuditObservation, SessionAuditObservationId, SessionAuditRecord, SpaceName,
};

pub(super) async fn complete_subagent(layout: &Layout, input: &str) -> Completion {
    let envelope = match harness_claude::SubagentStopEnvelope::parse(input) {
        Ok(Some(envelope)) => envelope,
        Ok(None) => return Completion::silent(),
        Err(error) => return Completion::refused(error.to_string()),
    };
    let selected = Layout::resolve_for_session(
        layout.project_dir(),
        envelope
            .session()
            .filter(|session| Layout::valid_session_id(session)),
    );
    let Some(path) = selected.state_file() else {
        return Completion::silent();
    };
    let Ok(bytes) = std::fs::read(path) else {
        return Completion::silent();
    };
    let state = String::from_utf8_lossy(&bytes);
    let status = field(&state, "Status").unwrap_or_default();
    let mut fields = AuditFields::new();
    for (key, value) in [
        ("Agent Type", envelope.agent_type()),
        ("Agent ID", envelope.agent_id()),
        ("Message", envelope.message()),
    ] {
        if key != "Agent Type" && value.is_empty() {
            continue;
        }
        let key = match AuditFieldKey::parse(key) {
            Ok(key) => key,
            Err(error) => return Completion::refused(error.to_string()),
        };
        fields = fields.with(key, value);
    }
    match record(layout, EventType::SubagentCompleted, fields, status).await {
        Ok(true) => {
            let _ = super::observe_hook_health(layout, "log-subagent").await;
        }
        Ok(false) => {}
        Err(error) => {
            let _ = super::observe_hook_health(layout, "log-subagent").await;
            let _ = super::record_hook_drop(layout, "log-subagent", &error).await;
        }
    }
    Completion::silent()
}

pub(super) async fn record(
    layout: &Layout,
    kind: EventType,
    fields: AuditFields,
    status: String,
) -> Result<bool, String> {
    let record = layout
        .record_dir()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .map(IntentDirName::parse)
        .transpose()
        .map_err(|error| error.to_string())?;
    let target = HookHealthTarget::new(
        SpaceName::parse(layout.space()).map_err(|error| format!("{error:?}"))?,
        record,
    );
    let observation_id = SessionAuditObservationId::generate();
    let observation = SessionAuditObservation::new(
        observation_id.clone(),
        target,
        SessionAuditRecord::new(kind, fields).map_err(|error| error.to_string())?,
        status,
    );
    let store = super::store_path(layout)?;
    let repository =
        core_command_interface_adapter::orchestration::SessionAuditRepositoryImpl::open(&store)
            .map_err(|error| error.to_string())?;
    core_command_use_case::orchestration::RecordSessionAuditUseCase::new(repository)
        .execute(&observation, Utc::now())
        .await
        .map_err(|error| error.to_string())?;
    super::catch_up(layout).await?;
    let daos = core_query_interface_adapter::ReadModelDaos::open(store.as_path())
        .map_err(|error| error.to_string())?;
    core_query_use_case::orchestration::SessionAuditUseCase::new(daos.session_audit())
        .execute(observation_id.as_str())
        .map(|result| result.is_some())
        .map_err(|error| error.to_string())
}

/// 状態ファイルの欄 `- **<name>**: <value>` を読む (本家 `getField`、`aidlc-lib.ts:16479`)。
///
/// 本家の正規表現は `^- \*\*<name>\*\*:[ \t]*(.*)$` (複数行モード) で、コロンの後の
/// 区切りは**空白かタブが 0 個以上**であり、一致した値を `trim()` する。`\s*` でなく
/// `[ \t]*` なのは、空の値が次の行を飲み込まないためである。最初に一致した行を採る。
pub(super) fn field(state: &str, name: &str) -> Option<String> {
    let prefix = format!("- **{name}**:");
    state
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .map(|value| core_infrastructure::ecmascript::trim(value).to_string())
}

pub(super) async fn end_session(layout: &Layout, input: &str) -> Completion {
    let envelope = harness_claude::SessionEndEnvelope::parse(input);
    let navigation = envelope.session().and_then(|session| {
        crate::session_navigation::SessionNavigation::new(layout.project_dir(), session)
    });
    let selected = if let Some(navigation) = navigation {
        if let Some(stamp) = navigation.stamp() {
            let Some(found) =
                crate::intent_location::IntentLocation::find(layout.project_dir(), &stamp)
            else {
                let reason = format!(
                    "session {} is stamped to unknown intent {stamp}; refusing active-cursor fallback",
                    envelope.session().unwrap_or_default()
                );
                let _ = super::record_hook_drop(layout, "session-end", &reason).await;
                return Completion::silent();
            };
            found.layout().clone()
        } else if crate::intent_location::IntentLocation::current(layout).is_some() {
            return Completion::silent();
        } else {
            layout.clone()
        }
    } else {
        layout.clone()
    };
    if selected.state_file().is_none_or(|path| !path.exists()) {
        return Completion::silent();
    }
    let heartbeat = super::observe_hook_health(&selected, "session-end").await;
    if heartbeat.code != 0 {
        return heartbeat;
    }
    let key = match AuditFieldKey::parse("Reason") {
        Ok(key) => key,
        Err(error) => return Completion::refused(error.to_string()),
    };
    let fields = AuditFields::new().with(key, envelope.reason());
    if let Err(error) = record(&selected, EventType::SessionEnded, fields, String::new()).await {
        let _ = super::record_hook_drop(&selected, "session-end", &error).await;
    }
    Completion::silent()
}

/// PreCompactの監査。状態を遷移させず、既存の当該シャードだけへ記録する。
pub(super) async fn validate_state(layout: &Layout, input: &str) -> Completion {
    let heartbeat = super::observe_hook_health(layout, "validate-state").await;
    if heartbeat.code != 0 {
        return heartbeat;
    }
    let Some(path) = layout.state_file().filter(|path| path.exists()) else {
        return Completion::silent();
    };
    let state = match std::fs::read(path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(error) => return Completion::refused(error.to_string()),
    };
    // 不正・欠落入力は文脈の所有を変えない。本家同様、失効処理の失敗は
    // 下の検証・復旧breadcrumb・監査を妨げない。
    if let Ok(payload) = serde_json::from_str::<serde_json::Value>(input)
        && let Some(session) = payload
            .get("session_id")
            .and_then(serde_json::Value::as_str)
            .or_else(|| payload.get("sessionId").and_then(serde_json::Value::as_str))
            .filter(|session| Layout::valid_session_id(session))
        && let Ok(Some(cursor)) = super::active_execution(layout)
    {
        let _ = super::plan_approval::invalidate_context(
            layout,
            cursor.execution_id(),
            cursor.intent_id(),
            session,
            &state,
        )
        .await;
    }
    // 復旧breadcrumbは機械ローカルな観測印であり、状態・承認の正本ではない。
    let missing: Vec<_> = ["Stage Progress", "Current Status"]
        .into_iter()
        .filter(|section| !state.contains(&format!("## {section}")))
        .collect();
    let state_status = if missing.is_empty() {
        "valid (all required sections present)".to_string()
    } else {
        format!("INVALID — missing sections: {}", missing.join(", "))
    };
    let current_stage = field(&state, "Current Stage").unwrap_or_default();
    let timestamp = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    if let Some(record) = layout.record_dir()
        && let Err(error) = std::fs::write(
            record.join(".aidlc-recovery.md"),
            format!(
                "# AIDLC Recovery Breadcrumb\n**Last validated**: {timestamp}\n**Current stage**: {current_stage}\n**State file**: {state_status}\n"
            ),
        )
    {
        return Completion::refused(error.to_string());
    }
    let fields = match AuditFieldKey::parse("Current Stage")
        .and_then(|stage| AuditFieldKey::parse("State Validity").map(|validity| (stage, validity)))
    {
        Ok((stage, validity)) => AuditFields::new().with(stage, &current_stage).with(
            validity,
            if missing.is_empty() {
                "valid"
            } else {
                "invalid"
            },
        ),
        Err(error) => return Completion::refused(error.to_string()),
    };
    if current_audit_exists(layout)
        && let Err(error) = record(layout, EventType::SessionCompacted, fields, String::new()).await
    {
        let _ = super::record_hook_drop(layout, "validate-state", &error).await;
    }
    if missing.is_empty() {
        Completion::silent()
    } else {
        Completion::new(
            None,
            Some(format!(
                "WARNING: aidlc-state.md missing sections: {}",
                missing.join(", ")
            )),
            0,
        )
    }
}

fn current_audit_exists(layout: &Layout) -> bool {
    let Some(directory) = layout.audit_dir() else {
        return false;
    };
    let Some(clone) = std::fs::read_to_string(layout.aidlc_root().join(".aidlc-clone-id"))
        .ok()
        .and_then(|text| core_command_domain::workspace::CloneId::parse(text.trim()).ok())
    else {
        return false;
    };
    directory
        .join(core_command_domain::workspace::ShardName::of(&super::host_name(), &clone).as_str())
        .exists()
}

#[cfg(test)]
mod tests {
    use super::field;

    #[test]
    fn a_field_is_read_with_any_number_of_spaces_or_tabs_after_the_colon() {
        // 本家 `getField` (`aidlc-lib.ts:16479`) は `^- \*\*Field\*\*:[ \t]*(.*)$` で読み、
        // 一致した値を `trim()` する。区切りは空白 1 個に限らない。
        for state in [
            "- **Current Stage**: code-generation\n",
            "- **Current Stage**:\tcode-generation\n",
            "- **Current Stage**:code-generation\n",
            "- **Current Stage**: \t  code-generation  \n",
        ] {
            assert_eq!(
                field(state, "Current Stage").as_deref(),
                Some("code-generation"),
                "{state:?}"
            );
        }
    }

    #[test]
    fn an_empty_value_is_the_empty_string_not_the_next_line() {
        // `[ \t]*` であって `\s*` でないのは、空の値が次の行を飲み込まないためである
        // (本家のコメント)。
        let state = "- **Current Stage**:\n- **Next Stage**: domain-design\n";
        assert_eq!(field(state, "Current Stage").as_deref(), Some(""));
        assert_eq!(field(state, "Next Stage").as_deref(), Some("domain-design"));
    }

    #[test]
    fn the_first_matching_line_wins_and_the_label_must_be_whole() {
        let state = "- **Stage Progress**: x\n- **Stage**: first\n- **Stage**: second\n";
        assert_eq!(field(state, "Stage").as_deref(), Some("first"));
        assert_eq!(field(state, "Missing"), None);
    }
}
