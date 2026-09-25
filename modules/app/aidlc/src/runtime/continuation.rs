//! Claude Stopから、読み取り用nextの照会と停止要求の保存を結線する。
use super::{
    Completion, Layout, ReadModelDaos, StorePath, Utc, active_execution, observe_hook_health,
    record_hook_drop,
};
use core_command_domain::orchestration::{
    ContinuationAttemptId, ContinuationRequest, ContinuationSignature, WorkflowContinuationId,
};
use core_command_interface_adapter::orchestration::WorkflowContinuationRepositoryImpl;
use core_command_use_case::orchestration::RecordContinuationUseCase;
use core_infrastructure::canon_json::{
    JsonValue, ObjectMembers, SerializationProfile, parse, serialize,
};
use core_query_use_case::orchestration::ContinuationResultUseCase;
use core_read_model_updater::orchestration::{
    ReadModelUpdater, WorkflowContinuationReadModelUpdater,
};

pub(super) async fn run(layout: &Layout, input: &str) -> Completion {
    let _ = observe_hook_health(layout, "continue-workflow").await;
    let envelope: serde_json::Value =
        serde_json::from_str(input).unwrap_or(serde_json::Value::Null);
    let session = envelope
        .get("session_id")
        .and_then(serde_json::Value::as_str)
        .filter(|session| Layout::valid_session_id(session));
    let selected = Layout::resolve_for_session(layout.project_dir(), session);
    match consider(&selected, input, session).await {
        Ok(completion) => completion,
        Err(error) => {
            let _ = record_hook_drop(layout, "continue-workflow", &error).await;
            Completion::silent()
        }
    }
}

async fn consider(
    layout: &Layout,
    input: &str,
    session: Option<&str>,
) -> Result<Completion, String> {
    let Some(state_path) = layout.state_file() else {
        return Ok(Completion::silent());
    };
    let state = std::fs::read_to_string(state_path).map_err(|error| error.to_string())?;
    if let Some(session) = session {
        let at = Utc::now();
        if layout.has_current_handoff(session, at) {
            let _ = layout.clear_handoff(session);
            let cursor = active_execution(layout)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "session handoff has no execution".to_string())?;
            let attempt = ContinuationAttemptId::generate();
            let request = ContinuationRequest::new(attempt.clone(), None, false, 1)
                .map_err(|error| error.to_string())?;
            let observations = core_command_domain::orchestration::ContinuationObservations::new(
                core_command_domain::orchestration::ContinuationQuestions::new(Vec::new()),
                false,
            );
            store_request(
                layout,
                cursor.execution_id(),
                &attempt,
                request,
                &observations,
            )
            .await?;
            let _ = record_hook_drop(
                layout,
                "continue-workflow",
                "allowing stop at the exact post-create fresh-session handoff boundary",
            )
            .await;
            return Ok(Completion::silent());
        }
        let _ = layout.discard_expired_handoff(session, at);
    }
    // 利用量の締め（本家 `aidlc-continue-workflow.ts:1328-1338`）— 状態ファイルが在り、
    // 切替印の境界でもないと分かった後、エンジンに問う前。失敗は Stop の判断を変えない。
    super::fold_usage::flush_on_stop(layout, session, input, &state);
    if shared_resume_waiting(layout, &state)? {
        let cursor = active_execution(layout)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "resume wait has no active execution".to_string())?;
        let attempt = ContinuationAttemptId::generate();
        let mut pending = ObjectMembers::new();
        pending.insert("kind", JsonValue::String("ask".into()));
        let request = ContinuationRequest::new(
            attempt.clone(),
            Some(
                ContinuationSignature::from_observation(&state, &JsonValue::Object(pending))
                    .map_err(|error| error.to_string())?,
            ),
            false,
            2,
        )
        .map_err(|error| error.to_string())?
        .with_wait_probe()
        .map_err(|error| error.to_string())?;
        let observations = core_command_domain::orchestration::ContinuationObservations::new(
            core_command_domain::orchestration::ContinuationQuestions::new(Vec::new()),
            false,
        )
        .with_resume_waiting(true);
        let result = store_request(
            layout,
            cursor.execution_id(),
            &attempt,
            request,
            &observations,
        )
        .await?;
        if result.wait() == Some("resume") {
            let _ = record_hook_drop(layout, "continue-workflow", "active resume choice is waiting on the human; allowing the stop before the shared next probe").await;
            return Ok(Completion::silent());
        }
    }
    let mut command =
        tokio::process::Command::new(std::env::current_exe().map_err(|error| error.to_string())?);
    command
        .arg("next")
        .arg("--project-dir")
        .arg(layout.project_dir())
        .current_dir(layout.project_dir())
        .env("AIDLC_STOP_HOOK_PROBE", "1")
        .kill_on_drop(true);
    if let Some(session) = session {
        command
            .env("AIDLC_SESSION_OVERRIDE", session)
            .env("AIDLC_SESSION_OVERRIDE_SOURCE", "payload");
    }
    let output = tokio::time::timeout(std::time::Duration::from_secs(10), command.output())
        .await
        .map_err(|_| "engine next returned no parseable directive; allowing stop".to_string())?
        .map_err(|_| "engine next returned no parseable directive; allowing stop".to_string())?;
    if !output.status.success() {
        return Err("engine next returned no parseable directive; allowing stop".into());
    }
    let directive = parse(core_infrastructure::ecmascript::trim(
        &String::from_utf8_lossy(&output.stdout),
    ))
    .map_err(|_| "engine next returned no parseable directive; allowing stop".to_string())?;
    let JsonValue::Object(fields) = &directive else {
        return Err("engine next returned no parseable directive; allowing stop".into());
    };
    let kind = text(fields, "kind");
    let reset = ["done", "notice", "parked"].contains(&kind);
    if kind == "ask" {
        return Ok(Completion::silent());
    }
    if kind.is_empty() {
        return Err("engine next returned no parseable directive; allowing stop".into());
    }
    let raw: serde_json::Value = serde_json::from_str(input).unwrap_or(serde_json::Value::Null);
    let reentrant = raw
        .get("stop_hook_active")
        .and_then(serde_json::Value::as_bool)
        == Some(true);
    let cursor = active_execution(layout)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "stop request has no active execution".to_string())?;
    let execution_daos = ReadModelDaos::open(super::store_path(layout)?.as_path())
        .map_err(|error| error.to_string())?;
    let execution = super::FindExecutionUseCase::new(execution_daos.execution())
        .execute(cursor.execution_id().as_str())
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "stop execution was not projected".to_string())?;
    let attempt = ContinuationAttemptId::generate();
    let request = ContinuationRequest::new(
        attempt.clone(),
        if reset {
            None
        } else {
            Some(
                ContinuationSignature::from_observation(&state, &directive)
                    .map_err(|error| error.to_string())?,
            )
        },
        !reset && reentrant,
        if reset { 1 } else { 2 },
    )
    .map_err(|error| error.to_string())?;
    let observations = core_command_domain::orchestration::ContinuationObservations::new(
        observe_questions(layout, &state, fields),
        observe_conversation(layout, &raw),
    );
    let result = store_request(
        layout,
        cursor.execution_id(),
        &attempt,
        request,
        &observations,
    )
    .await?;
    if reset {
        return Ok(Completion::silent());
    }
    if let Some(wait) = result.wait() {
        let stage = execution.cursor_slug().unwrap_or_default();
        let reason = match wait {
            "gate-or-revision" => format!("current stage {stage} is awaiting approval or being revised; allowing the stop (human-wait carve-out)"),
            "decision" => format!("current stage {stage} has an unanswered logged decision; allowing the stop (pending-decision carve-out)"),
            "conversation" => "the ending turn was conversational (human's last prompt answered with no workflow-engine call); allowing the stop (conversational carve-out)".to_string(),
            "question" => format!(
                "active stage {} has an unanswered question; allowing the stop (pending-question carve-out)",
                text(fields, "stage")
            ),
            _ => return Err("unknown projected continuation wait".into()),
        };
        let _ = record_hook_drop(layout, "continue-workflow", &reason).await;
        return Ok(Completion::silent());
    }
    if !result.blocked() {
        let _ = record_hook_drop(layout, "continue-workflow", &format!("recursion guard released the stop (no-progress block cap {} reached; stop_hook_active={reentrant})", result.limit())).await;
        return Ok(Completion::silent());
    }
    let mut answer = ObjectMembers::new();
    answer.insert("decision", JsonValue::String("block".into()));
    answer.insert("reason", JsonValue::String(reason(fields)));
    Ok(Completion::emitted(serialize(
        &JsonValue::Object(answer),
        SerializationProfile::ContractCompact,
    )))
}

fn observe_questions(
    layout: &Layout,
    state: &str,
    directive: &ObjectMembers,
) -> core_command_domain::orchestration::ContinuationQuestions {
    use core_command_domain::orchestration::{ContinuationQuestion, ContinuationQuestions};
    let empty = || ContinuationQuestions::new(Vec::new());
    let Some(record) = layout.record_dir() else {
        return empty();
    };
    let phase = state
        .lines()
        .find_map(|line| line.strip_prefix("- **Lifecycle Phase**:"))
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if ![
        "initialization",
        "ideation",
        "inception",
        "construction",
        "operation",
    ]
    .contains(&phase.as_str())
    {
        return empty();
    }
    let stage = text(directive, "stage");
    if core_command_domain::workflow_definition::StageSlug::parse(stage).is_err() {
        return empty();
    }
    let mut directory = record.join(&phase);
    let unit = text(directive, "unit");
    if phase == "construction" && !unit.is_empty() {
        if std::path::Path::new(unit).components().count() != 1
            || !matches!(
                std::path::Path::new(unit).components().next(),
                Some(std::path::Component::Normal(_))
            )
        {
            return empty();
        }
        directory = directory.join(unit);
    }
    directory = directory.join(stage);
    let Ok(files) = std::fs::read_dir(directory) else {
        return empty();
    };
    ContinuationQuestions::new(
        files
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .ends_with("-questions.md")
            })
            .filter_map(|entry| std::fs::read(entry.path()).ok())
            .map(|bytes| ContinuationQuestion::new(String::from_utf8_lossy(&bytes).into_owned()))
            .collect(),
    )
}

fn text<'a>(fields: &'a ObjectMembers, key: &str) -> &'a str {
    match fields.get(key) {
        Some(JsonValue::String(value)) => core_infrastructure::ecmascript::trim(value),
        _ => "",
    }
}

fn shared_resume_waiting(layout: &Layout, state: &str) -> Result<bool, String> {
    use std::io::Read as _;
    let Some(record) = layout.record_dir() else {
        return Ok(false);
    };
    let path = record.join(".aidlc-active-directive.json");
    if !path.exists() {
        return Ok(false);
    }
    // nativeの指示・承認更新と同じ排他を取り、1 descriptorの上限付き読取を行う。
    let lock_file = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(layout.aidlc_root().join(".aidlc-runtime.lock")).map_err(|error| format!("active-directive evidence unavailable while reading shared resume wait: {error}; allowing stop"))?;
    let _lock = core_infrastructure::ExclusiveFileLock::acquire(lock_file, std::time::Duration::from_secs(1)).map_err(|error| {
        let cause = if error.kind() == std::io::ErrorKind::WouldBlock { "Active-directive coordination is busy".to_string() } else { error.to_string() };
        format!("active-directive evidence unavailable while reading shared resume wait: {cause}; allowing stop")
    })?;
    let Ok(file) = std::fs::File::open(path) else {
        return Ok(false);
    };
    let mut bytes = Vec::new();
    if file.take(65_537).read_to_end(&mut bytes).is_err() || bytes.len() > 65_536 {
        return Ok(false);
    }
    let stable_state = layout
        .state_file()
        .and_then(|path| std::fs::read(path).ok())
        .is_some_and(|current| current == state.as_bytes());
    Ok(stable_state
        && harness_claude::StopResumeWait::parse(&String::from_utf8_lossy(&bytes))
            .matches_state_hash(&core_infrastructure::hash::sha256_hex(state.as_bytes())))
}

fn reason(fields: &ObjectMembers) -> String {
    let kind = text(fields, "kind");
    let stage = text(fields, "stage");
    let context = if stage.is_empty() {
        String::new()
    } else {
        format!(" for \"{stage}\"")
    };
    let token = text(fields, "continue_token");
    if kind == "load-steering" && !token.is_empty() {
        let rules = fields
            .get("rules_content")
            .cloned()
            .unwrap_or_else(|| JsonValue::Array(Vec::new()));
        return format!(
            "The AIDLC workflow still has rules to load{context}. Preserve this step-two continuation command, but do not run it yet: `bun .claude/tools/aidlc-orchestrate.ts continue \"{token}\"` First, apply every path/text entry in the exact `rules_content` payload below. Second, run the preserved command and keep following each load-steering step it returns, applying its rule chunk before every continuation, until it answers `run-stage`. Do not summarise or narrate these rule chunks to the user.\n\n{}",
            serialize(&rules, SerializationProfile::ContractCompact)
        );
    }
    format!(
        "The AIDLC workflow has a pending step (a {kind} directive{context}). You have not finished the workflow loop yet. Run `bun .claude/tools/aidlc-orchestrate.ts next`, do what the step it prints asks, then run `aidlc-orchestrate report --stage <stage> --result <outcome>` to record the outcome. Repeat until it answers `done`. If you meant to pause this workflow instead and pick it up in a later session, run `bun .claude/tools/aidlc-orchestrate.ts park` to stop cleanly between stages - never mark a stage complete just to end the turn."
    )
}

fn observe_conversation(layout: &Layout, envelope: &serde_json::Value) -> bool {
    if let Some(path) = envelope
        .get("transcript_path")
        .and_then(serde_json::Value::as_str)
        .filter(|path| !path.is_empty())
    {
        return std::fs::read(path).ok().is_some_and(|bytes| {
            harness_claude::StopTranscript::parse(&String::from_utf8_lossy(&bytes))
                .is_conversational()
        });
    }
    let Some(record) = layout.record_dir() else {
        return false;
    };
    let (Ok(human), Ok(engine)) = (
        std::fs::metadata(record.join(".aidlc-human-turn")),
        std::fs::metadata(record.join(".aidlc-engine-touch")),
    ) else {
        return false;
    };
    if !human.is_file() || !engine.is_file() {
        return false;
    }
    matches!((human.modified(), engine.modified()), (Ok(human), Ok(engine)) if human > engine)
}

async fn store_request(
    layout: &Layout,
    execution_id: &core_command_domain::orchestration::IntentExecutionId,
    attempt: &ContinuationAttemptId,
    request: ContinuationRequest,
    observations: &core_command_domain::orchestration::ContinuationObservations,
) -> Result<core_query_use_case::orchestration::ContinuationResultView, String> {
    let id = WorkflowContinuationId::for_execution(execution_id);
    let store = StorePath::for_runtime(&layout.aidlc_root());
    super::plan_approval::prepare_hook_store(layout)?;
    let lock_file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(layout.aidlc_root().join(".aidlc-runtime.lock"))
        .map_err(|error| error.to_string())?;
    let _lock = core_infrastructure::ExclusiveFileLock::acquire(
        lock_file,
        std::time::Duration::from_secs(1),
    )
    .map_err(|error| error.to_string())?;
    recover_publication(layout, &store, &id).await?;
    let request = request.with_observed_guard(observe_counter(layout)?);
    let repository =
        WorkflowContinuationRepositoryImpl::open(&store).map_err(|error| error.to_string())?;
    let executions = super::IntentExecutionRepositoryImpl::open(&super::store_path(layout)?)
        .map_err(|error| error.to_string())?;
    let limit_override = std::env::var("CLAUDE_CODE_STOP_HOOK_BLOCK_CAP").ok();
    RecordContinuationUseCase::new(repository, executions)
        .execute(
            execution_id,
            request,
            limit_override.as_deref(),
            observations,
            Utc::now(),
        )
        .await
        .map_err(|error| error.to_string())?;
    recover_publication(layout, &store, &id).await?;
    let daos = ReadModelDaos::open(store.as_path()).map_err(|error| error.to_string())?;
    let result = ContinuationResultUseCase::new(daos.continuation_result())
        .execute(attempt.as_str())
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "stop result was not projected".to_string())?;
    Ok(result)
}

async fn recover_publication(
    layout: &Layout,
    store: &StorePath,
    id: &WorkflowContinuationId,
) -> Result<(), String> {
    let record = layout
        .record_dir()
        .ok_or_else(|| "stop request has no record".to_string())?;
    let mut updater = WorkflowContinuationReadModelUpdater::open(
        store.as_path(),
        id.clone(),
        record.to_path_buf(),
    )
    .map_err(|error| error.to_string())?;
    updater
        .update_read_models()
        .await
        .map_err(|error| error.to_string())?;
    let daos = ReadModelDaos::open(store.as_path()).map_err(|error| error.to_string())?;
    let query = ContinuationResultUseCase::new(daos.continuation_result());
    let Some(pending) = query
        .unsettled(id.as_str())
        .map_err(|error| error.to_string())?
    else {
        return Ok(());
    };
    let attempt = ContinuationAttemptId::parse(pending.id()).map_err(|error| error.to_string())?;
    let published = pending
        .published()
        .ok_or_else(|| "continuation publication observation is missing".to_string())?;
    core_command_use_case::orchestration::SettleContinuationPublicationUseCase::new(
        WorkflowContinuationRepositoryImpl::open(store).map_err(|error| error.to_string())?,
    )
    .execute(
        id,
        &core_command_domain::orchestration::ContinuationPublicationObservation::new(
            attempt.clone(),
            published,
        ),
        Utc::now(),
    )
    .await
    .map_err(|error| error.to_string())?;
    updater
        .update_read_models()
        .await
        .map_err(|error| error.to_string())?;
    let settled = query
        .execute(attempt.as_str())
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "continuation settlement was not projected".to_string())?;
    if !settled.settled() {
        return Err("continuation settlement is incomplete".into());
    }
    Ok(())
}

fn observe_counter(
    layout: &Layout,
) -> Result<core_command_domain::orchestration::ContinuationGuard, String> {
    use core_command_domain::orchestration::ContinuationGuard;
    let empty = || ContinuationGuard::new(None, 0, false).map_err(|error| error.to_string());
    let Some(record) = layout.record_dir() else {
        return empty();
    };
    let parsed = std::fs::read(record.join(".aidlc-stop-hook/block-count.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
    let Some(value) = parsed else {
        return empty();
    };
    let (Some(signature), Some(count)) = (
        value.get("signature").and_then(serde_json::Value::as_str),
        value.get("count").and_then(serde_json::Value::as_u64),
    ) else {
        return empty();
    };
    let signature = if signature.is_empty() {
        None
    } else {
        match ContinuationSignature::parse(signature) {
            Ok(signature) => Some(signature),
            Err(_) => return empty(),
        }
    };
    match ContinuationGuard::new(signature, count, true) {
        Ok(guard) => Ok(guard),
        Err(_) => empty(),
    }
}
