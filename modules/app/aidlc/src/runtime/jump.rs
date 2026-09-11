//! 本家aidlc-jumpの通常root execute接続。
use super::{Completion, Layout, active_execution, catch_up, store_path};
use core_command_domain::{orchestration::IntentExecutionEventId, workflow_definition::StageSlug};
use core_command_interface_adapter::orchestration::{
    IntentExecutionRepositoryImpl, IntentRepositoryImpl, WorkflowDefinitionRepositoryImpl,
};
use core_command_use_case::orchestration::JumpUseCase;
use core_query_interface_adapter::ReadModelDaos;
use core_query_use_case::orchestration::JumpResultUseCase;
pub(super) async fn run(layout: &Layout, args: &[String]) -> Completion {
    match execute(layout, args).await {
        Ok(output) => Completion::emitted(output),
        Err(message) => Completion::refused(message),
    }
}
async fn execute(layout: &Layout, args: &[String]) -> Result<String, String> {
    let verb = args.first().map_or("undefined", String::as_str);
    if !["execute", "resolve"].contains(&verb) {
        return Err(format!(
            "Unknown subcommand: {verb}. Valid: resolve, execute"
        ));
    }
    let mut flags = std::collections::BTreeMap::new();
    let mut arguments = args.iter().skip(1);
    while let Some(argument) = arguments.next() {
        if let Some(key) = argument.strip_prefix("--")
            && let Some(value) = arguments.next()
        {
            flags.insert(key.to_string(), value.clone());
        }
    }
    if verb == "resolve" {
        return resolve(layout, &flags).await;
    }
    let target = flags
        .get("target")
        .filter(|value| !value.is_empty())
        .ok_or(
            "Usage: execute --target <slug> --direction <forward|backward|redo> [--scope <scope>]",
        )?;
    let direction = flags.get("direction").map_or("undefined", String::as_str);
    if !["forward", "backward", "redo"].contains(&direction) {
        return Err(format!(
            "Invalid direction: {direction}. Valid: forward, backward, redo"
        ));
    }
    let direction = match direction {
        "forward" => core_command_domain::orchestration::JumpDirection::Forward,
        "backward" => core_command_domain::orchestration::JumpDirection::Backward,
        _ => core_command_domain::orchestration::JumpDirection::Redo,
    };
    let cursor = active_execution(layout)
        .map_err(|error| error.to_string())?
        .ok_or("No active workflow is selected")?;
    let store = store_path(layout)?;
    let target = StageSlug::parse(target).map_err(|_| format!("Unknown stage: {target}"))?;
    let id = IntentExecutionEventId::generate();
    let baseline =
        crate::source_baseline::read(layout.project_dir()).map_err(|error| error.to_string())?;
    let observation =
        core_command_domain::orchestration::JumpObservation::new(baseline, artifacts(layout)?);
    JumpUseCase::new(
        IntentExecutionRepositoryImpl::open(&store).map_err(|error| error.to_string())?,
        IntentRepositoryImpl::open(&store).map_err(|error| error.to_string())?,
        WorkflowDefinitionRepositoryImpl::open(&store).map_err(|error| error.to_string())?,
    )
    .execute(
        cursor.execution_id(),
        &id,
        &target,
        direction,
        flags
            .get("scope")
            .map(String::as_str)
            .filter(|value| !value.is_empty()),
        observation,
        chrono::Utc::now(),
    )
    .await
    .map_err(|error| error.to_string())?;
    catch_up(layout).await?;
    let view = JumpResultUseCase::new(
        ReadModelDaos::open(store.as_path())
            .map_err(|error| error.to_string())?
            .jump_result(),
    )
    .execute(id.as_str())
    .map_err(|error| error.to_string())?
    .ok_or("Saved jump result was not projected")?;
    Ok(view.payload().to_string())
}

fn artifacts(
    layout: &Layout,
) -> Result<Vec<core_command_domain::orchestration::JumpArtifact>, String> {
    use core_command_domain::{orchestration::JumpArtifact, workspace::extract_section};
    let graph: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            layout
                .project_dir()
                .join(".claude/tools/data/stage-graph.json"),
        )
        .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let nodes = graph.as_array().ok_or("stage graph must be an array")?;
    let record = layout.record_dir().ok_or("active record unavailable")?;
    let mut observed = Vec::new();
    for node in nodes {
        let slug = node
            .get("slug")
            .and_then(serde_json::Value::as_str)
            .ok_or("stage slug unavailable")?;
        let stage = StageSlug::parse(slug).map_err(|e| e.to_string())?;
        let phase = node
            .get("phase")
            .and_then(serde_json::Value::as_str)
            .ok_or("stage phase unavailable")?;
        for name in ["produces", "optional_produces"]
            .into_iter()
            .flat_map(|key| {
                node.get(key)
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
            })
        {
            let filename = name
                .as_str()
                .ok_or("artifact name must be text")?
                .rsplit('/')
                .next()
                .ok_or("artifact filename unavailable")?;
            let filename = if std::path::Path::new(filename).extension().is_some() {
                filename.to_string()
            } else {
                format!("{filename}.md")
            };
            let roots = if slug == "reverse-engineering" {
                let codekb = layout
                    .project_dir()
                    .join("aidlc/spaces")
                    .join(layout.space())
                    .join("codekb");
                let repos = if codekb.exists() {
                    std::fs::read_dir(&codekb)
                        .map_err(|e| e.to_string())?
                        .map(|entry| entry.map(|entry| entry.path()))
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|error| error.to_string())?
                        .into_iter()
                        .filter(|path| path.is_dir())
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                if repos.is_empty() {
                    observed.push(JumpArtifact::new(
                        stage.clone(),
                        format!("codekb/*/{filename}"),
                        false,
                        false,
                    ));
                    continue;
                }
                repos
            } else {
                vec![record.join(phase).join(slug)]
            };
            for root in roots {
                let path = root.join(&filename);
                let exists = path.exists();
                let review = std::fs::read_to_string(&path)
                    .ok()
                    .and_then(|text| extract_section(&text, "## Review"))
                    .is_some_and(|section| !section.is_empty());
                let relative = path
                    .strip_prefix(layout.project_dir())
                    .map_err(|e| e.to_string())?
                    .to_str()
                    .ok_or("artifact path is not UTF-8")?
                    .to_string();
                observed.push(JumpArtifact::new(stage.clone(), relative, exists, review));
            }
        }
    }
    Ok(observed)
}

async fn resolve(
    layout: &Layout,
    flags: &std::collections::BTreeMap<String, String>,
) -> Result<String, String> {
    use core_query_use_case::orchestration::FindJumpUseCase;
    let cursor = active_execution(layout)
        .map_err(|e| e.to_string())?
        .ok_or("No active workflow is selected")?;
    catch_up(layout).await?;
    let store = store_path(layout)?;
    let daos = ReadModelDaos::open(store.as_path()).map_err(|e| e.to_string())?;
    let query = FindJumpUseCase::new(daos.jump(), daos.jump_phase());
    let view = if let Some(stage) = flags.get("stage").filter(|s| !s.is_empty()) {
        query
            .execute(cursor.execution_id().as_str(), stage)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Unknown stage: {stage}"))?
    } else if let Some(phase) = flags.get("phase").filter(|s| !s.is_empty()) {
        query
            .execute_phase(cursor.execution_id().as_str(), &phase.to_lowercase())
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Unknown phase: {phase}"))?
    } else {
        return Err("Usage: resolve --stage <slug|#> or --phase <name|#> [--scope <scope>]".into());
    };
    view.resolution().map(str::to_string).ok_or_else(|| {
        format!(
            "Stage \"{}\" cannot be reached from this workflow",
            view.target_slug()
        )
    })
}
