//! テスト契約の公開入口。規則の解釈はRMUから呼ぶドメインが担う。
use super::{Completion, Layout, active_execution, store_path};
use core_infrastructure::canon_json::{JsonValue, ObjectMembers, SerializationProfile, serialize};
use core_query_interface_adapter::ReadModelDaos;
use core_query_use_case::orchestration::{FindTestingContractUseCase, TestingContractView};
use core_read_model_updater::orchestration::{JournalReaderImpl, ReadModelUpdater, SteeringSource};

pub(super) async fn run(layout: &Layout, args: &[String]) -> Completion {
    let command = args
        .iter()
        .find(|arg| {
            matches!(
                arg.as_str(),
                "resolve" | "render" | "fingerprint" | "verify" | "begin"
            )
        })
        .map(String::as_str);
    let result = match command {
        Some("render" | "resolve") => load(layout).await.and_then(|view| {
            if let Some(error) = view.error() {
                return Err(error.to_string());
            }
            let text = if command == Some("render") {
                view.rendered()
            } else {
                view.contract()
            };
            text.map(|text| text.strip_suffix('\n').unwrap_or(text).to_string())
                .ok_or_else(|| "Projected Testing Contract is incomplete".to_string())
        }),
        Some("fingerprint") => fingerprint(layout, args).await,
        Some("begin") => begin(layout, args).await,
        Some(command) => Err(format!("Testing Posture {command} is not connected")),
        None => Err(
            "Unknown subcommand: (none). Valid: resolve, render, fingerprint, verify, begin"
                .to_string(),
        ),
    };
    match result {
        Ok(text) => Completion::emitted(text),
        Err(message) => failure(&message),
    }
}
pub(super) async fn load(layout: &Layout) -> Result<TestingContractView, String> {
    let store = store_path(layout)?;
    let mut reader = JournalReaderImpl::open(&store).map_err(|error| error.to_string())?;
    ReadModelUpdater::catch_up_testing(&mut reader, &SteeringSource::new(layout.memory_dir()))
        .await
        .map_err(|error| error.to_string())?;
    let cursor = active_execution(layout).map_err(|error| error.to_string())?;
    let id = cursor
        .as_ref()
        .map_or("bare-space", |cursor| cursor.intent_id().as_str());
    FindTestingContractUseCase::new(
        ReadModelDaos::open(store.as_path())
            .map_err(|error| error.to_string())?
            .testing_contract(),
    )
    .execute(id)
    .map_err(|error| error.to_string())?
    .ok_or_else(|| "Projected Testing Contract is missing".to_string())
}
fn failure(message: &str) -> Completion {
    let mut fields = ObjectMembers::new();
    fields.insert("error", JsonValue::String(message.to_string()));
    Completion::refused(serialize(
        &JsonValue::Object(fields),
        SerializationProfile::ContractCompact,
    ))
}

pub(super) fn target(
    args: &[String],
    command: &str,
) -> Result<core_command_domain::orchestration::PlanTarget, String> {
    use core_command_domain::orchestration::PlanTarget;
    let unit = args.iter().position(|arg| arg == "--unit");
    let stage = args.iter().any(|arg| arg == "--stage-level");
    if unit.is_some() && stage {
        return Err(format!(
            "{command} accepts exactly one of --unit <unit> or --stage-level"
        ));
    }
    if let Some(index) = unit {
        let value = args
            .get(index + 1)
            .filter(|value| {
                !value.starts_with("--") && !core_infrastructure::ecmascript::trim(value).is_empty()
            })
            .ok_or_else(|| format!("{command} requires a non-blank --unit <unit>"))?;
        return PlanTarget::for_unit(value).map_err(|error| error.to_string());
    }
    if stage {
        Ok(PlanTarget::stage_level())
    } else {
        Err(format!(
            "{command} requires exactly one of --unit <unit> or --stage-level"
        ))
    }
}
async fn fingerprint(layout: &Layout, args: &[String]) -> Result<String, String> {
    use core_query_use_case::orchestration::FindPlanFingerprintUseCase;
    use core_read_model_updater::orchestration::PlanSource;
    let target = target(args, "fingerprint")?;
    let record = layout.record_dir().ok_or_else(|| {
        "Code Generation approval authority requires an active workflow state".to_string()
    })?;
    let cursor = active_execution(layout).map_err(|error| error.to_string())?.ok_or_else(|| "Code Generation approval authority is unavailable because the active directive is missing, stale, or legacy; run a fresh `next`".to_string())?;
    let input = PlanSource::new(
        layout.project_dir().to_path_buf(),
        record.to_path_buf(),
        layout.memory_dir(),
        target.clone(),
    )
    .read(None)
    .map_err(|error| error.to_string())?;
    let store = store_path(layout)?;
    let mut reader = JournalReaderImpl::open(&store).map_err(|error| error.to_string())?;
    ReadModelUpdater::catch_up_plan_fingerprint(&mut reader, cursor.execution_id(), &input)
        .await
        .map_err(|error| error.to_string())?;
    let daos = ReadModelDaos::open(store.as_path()).map_err(|error| error.to_string())?;
    let view = FindPlanFingerprintUseCase::new(daos.plan_fingerprint())
        .execute(cursor.execution_id().as_str(), &target.id())
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Projected plan fingerprint is missing".to_string())?;
    if let Some(error) = view.error() {
        return Err(error.to_string());
    }
    view.fingerprint()
        .map(str::to_string)
        .ok_or_else(|| "Projected plan fingerprint is incomplete".to_string())
}

async fn begin(layout: &Layout, args: &[String]) -> Result<String, String> {
    use core_command_domain::orchestration::PlanApprovalOperationId;
    use core_command_domain::workspace::StorePath;
    use core_query_use_case::orchestration::PlanGenerationUseCase;
    use core_read_model_updater::orchestration::PlanSource;
    let target = target(args, "begin")?;
    let record = layout
        .record_dir()
        .ok_or("Code Generation approval authority requires an active workflow state")?;
    let cursor=active_execution(layout).map_err(|error|error.to_string())?.ok_or("Code Generation approval authority is unavailable because the active directive is missing, stale, or legacy; run a fresh `next`")?;
    let mut access = super::plan_approval::PlanApprovalAccess::open(layout).await?;
    let source = crate::source_fingerprint::read(layout.project_dir()).ok();
    let input = PlanSource::new(
        layout.project_dir().to_path_buf(),
        record.to_path_buf(),
        layout.memory_dir(),
        target,
    )
    .read(source)
    .map_err(|error| error.to_string())?;
    let id = PlanApprovalOperationId::generate();
    access
        .begin(layout, id.clone(), cursor.execution_id(), &input)
        .await?;
    let store = StorePath::for_runtime(&layout.aidlc_root());
    let daos = ReadModelDaos::open(store.as_path()).map_err(|error| error.to_string())?;
    let result = PlanGenerationUseCase::new(daos.plan_generation())
        .execute(id.as_str())
        .map_err(|error| error.to_string())?
        .ok_or("Generation result projection is missing")?;
    if let Some(error) = result.error() {
        return Err(error.to_string());
    }
    if result.status() != "generation" {
        return Err("Generation publication is not certified".to_string());
    }
    let mut fields = ObjectMembers::new();
    fields.insert("status", JsonValue::String(result.status().to_string()));
    let mut target = ObjectMembers::new();
    target.insert(
        "unit",
        result
            .unit()
            .map_or(JsonValue::Null, |unit| JsonValue::String(unit.to_string())),
    );
    fields.insert("target", JsonValue::Object(target));
    Ok(serialize(
        &JsonValue::Object(fields),
        SerializationProfile::ContractCompact,
    ))
}
