//! log linkの構文・ファイル観測・保存・応答の接続。
use super::{Completion, Layout, active_execution, after_projection, store_path};
use crate::cli::LinkArgs;
use core_command_domain::orchestration::{
    PipelineHandoff, PipelineHandoffInput, PipelineLinkError, PipelineLinkRequest,
};
use core_command_interface_adapter::orchestration::{
    IntentExecutionRepositoryImpl, IntentRepositoryImpl, WorkflowDefinitionRepositoryImpl,
};
use core_command_use_case::orchestration::{PipelineLinkCommandError, RecordPipelineLinkUseCase};
use core_infrastructure::canon_json::{JsonValue, ObjectMembers, SerializationProfile, serialize};
use core_query_use_case::orchestration::dispatcher_invocation;
use std::{
    fs,
    io::Read as _,
    path::{Component, Path, PathBuf},
};

pub(super) async fn run(layout: &Layout, args: &LinkArgs) -> Completion {
    if let Some(error) = args.parse_error() {
        return Completion::refused(error.to_string());
    }
    let Some(stage) = args.value("stage").filter(|s| !s.is_empty()) else {
        return Completion::refused("Missing --stage <slug>".into());
    };
    let Some(link) = args.value("link").filter(|s| !s.is_empty()) else {
        return Completion::refused("Missing --link <agent>".into());
    };
    if ["intent", "space"]
        .iter()
        .any(|key| args.value(key).is_some_and(|value| !value.is_empty()))
    {
        return Completion::refused("The link command does not accept --intent/--space selectors. Switch to the target workspace first.".into());
    }
    // record と state ファイルは同じ配置から導く (`Layout::state_file`) ので一度に確かめる。
    let Some(record) = layout
        .record_dir()
        .filter(|_| layout.state_file().is_some_and(|path| path.exists()))
    else {
        return Completion::refused("No active workflow is selected, so this interaction cannot be recorded. Start one by describing what to build (/aidlc \"build the auth service\"), or switch to an existing one with /aidlc intent <name>.".into());
    };
    let cursor = match active_execution(layout) {
        Ok(Some(cursor)) => cursor,
        Ok(None) => {
            return Completion::refused(
                "Cannot resolve the active intent for pipeline link logging.".into(),
            );
        }
        Err(error) => return Completion::refused(error.to_string()),
    };
    let name = args
        .value("repo")
        .filter(|repo| !repo.is_empty())
        .map_or_else(
            || "developer-scan.md".to_string(),
            |repo| format!("developer-scan-{repo}.md"),
        );
    let expected = record.join("inception/reverse-engineering").join(name);
    let supplied = args.value("artifact").map(str::to_string);
    let same = supplied
        .as_ref()
        .is_some_and(|raw| normalize(&layout.project_dir().join(raw)) == normalize(&expected));
    let relative = expected
        .strip_prefix(layout.project_dir())
        .unwrap_or(&expected)
        .to_string_lossy()
        .into_owned();
    let observed = observe(layout.project_dir(), &expected, &relative);
    let request = PipelineLinkRequest::new(
        stage.into(),
        link.into(),
        args.value("repo").map(str::to_string),
        args.value("single") == Some("true"),
        PipelineHandoffInput::new(supplied, relative, same, observed),
    );
    let store = match store_path(layout) {
        Ok(store) => store,
        Err(error) => return Completion::refused(error),
    };
    let (Ok(executions), Ok(intents), Ok(definitions)) = (
        IntentExecutionRepositoryImpl::open(&store),
        IntentRepositoryImpl::open(&store),
        WorkflowDefinitionRepositoryImpl::open(&store),
    ) else {
        return Completion::refused("Cannot open the pipeline link repositories.".into());
    };
    if let Err(error) = RecordPipelineLinkUseCase::new(executions, intents, definitions)
        .execute(cursor.execution_id(), &request, chrono::Utc::now())
        .await
    {
        return Completion::refused(match error {
            PipelineLinkCommandError::Rejected(error) => wording(&error),
            error => error.to_string(),
        });
    }
    after_projection(layout, || {
        let mut fields = ObjectMembers::new();
        fields.insert(
            "emitted",
            JsonValue::String("PIPELINE_LINK_COMPLETED".into()),
        );
        fields.insert("stage", JsonValue::String(stage.into()));
        fields.insert("link", JsonValue::String(link.into()));
        if let Some(repo) = args.value("repo").filter(|repo| !repo.is_empty()) {
            fields.insert("repo", JsonValue::String(repo.into()));
        }
        if args.value("single") == Some("true") {
            fields.insert("single", JsonValue::Bool(true));
        }
        Completion::emitted(serialize(
            &JsonValue::Object(fields),
            SerializationProfile::ContractCompact,
        ))
    })
    .await
}
fn normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            component => result.push(component.as_os_str()),
        }
    }
    result
}
fn observe(project: &Path, path: &Path, relative: &str) -> Result<Option<PipelineHandoff>, String> {
    if Path::new(relative)
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("handoff path is outside the selected record".to_string());
    }
    if !path.exists() {
        return Ok(None);
    }
    let root = fs::canonicalize(project).map_err(|e| e.to_string())?;
    let mut guarded = root;
    for part in Path::new(relative).components() {
        guarded.push(part);
        if fs::symlink_metadata(&guarded)
            .map_err(|e| e.to_string())?
            .is_symlink()
        {
            return Err(format!(
                "{} is a symlink, and no path component may be a symlink here. A redirected container directory is refused exactly like a symlink found INSIDE an already-trusted one.",
                guarded.display()
            ));
        }
    }
    let metadata = fs::symlink_metadata(&guarded).map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err(format!(
            "reverse-engineering developer handoff is not a regular file ({}): {}. Only regular files are read — a FIFO, socket, or device file can block forever or never reach EOF, so it is refused before any read.",
            if metadata.is_dir() {
                "a directory"
            } else {
                "a special file"
            },
            guarded.display()
        ));
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let mut file = options.open(&guarded).map_err(|e| e.to_string())?;
    let before = file.metadata().map_err(|e| e.to_string())?;
    if !before.is_file() {
        return Err(format!(
            "reverse-engineering developer handoff is not a regular file: {}",
            guarded.display()
        ));
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    let after = file.metadata().map_err(|e| e.to_string())?;
    if before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
        || crate::source_fingerprint::metadata_changed(&before, &after)
    {
        return Err(format!(
            "reverse-engineering developer handoff changed while reading: {}",
            guarded.display()
        ));
    }
    let modified = before
        .modified()
        .map_err(|e| e.to_string())?
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?;
    PipelineHandoff::new(
        relative.into(),
        format!("sha256:{}", core_infrastructure::hash::sha256_hex(&bytes)),
        mtime_ms(modified).to_string(),
    )
    .map(Some)
    .map_err(|e| e.to_string())
}
#[expect(
    clippy::cast_precision_loss,
    reason = "本家stat.mtimeMsは秒とナノ秒をNumberのミリ秒へ変換する"
)]
fn mtime_ms(time: std::time::Duration) -> f64 {
    time.as_secs() as f64 * 1000.0 + f64::from(time.subsec_nanos()) / 1_000_000.0
}
fn wording(error: &PipelineLinkError) -> String {
    use PipelineLinkError as E;
    match error {
        E::NotPipeline{stage}=>format!("Cannot record pipeline link: stage \"{stage}\" is not mode: pipeline."),
        E::UnknownLink{stage,link,declared}=>format!("Cannot record pipeline link for \"{stage}\": \"{link}\" is not in its declared lead/support chain ({declared})."),
        E::UnregisteredRepo{stage}=>format!("Cannot record pipeline link for \"{stage}\": this intent has no registered repo identity; omit --repo."),
        E::Duplicate{stage,link,repo}=>format!("Cannot record pipeline link for \"{stage}\": link \"{link}\"{} already completed this attempt.",repo.as_ref().filter(|s|!s.is_empty()).map_or(String::new(),|r|format!(" for repo \"{r}\""))),
        E::OutOfOrder{stage,link,previous,position,total,repo}=>format!("Cannot record pipeline link for \"{stage}\": \"{link}\" is out of order; position {position}/{total} requires current-attempt receipt for \"{previous}\"{}.",repo.as_ref().filter(|s|!s.is_empty()).map_or(String::new(),|r|format!(" in repo \"{r}\""))),
        E::ArtifactRequired=>"Cannot record reverse-engineering developer link: pass --artifact \"<record>/inception/reverse-engineering/developer-scan[-<repo>].md\".".into(),
        E::ArtifactPath{expected}=>format!("Cannot record reverse-engineering developer link: --artifact must resolve to {expected}."),
        E::ArtifactMissing{supplied}=>format!("Cannot record reverse-engineering developer link: handoff file does not exist: {supplied}."),
        E::ArtifactUnreadable{cause}=>format!("Cannot record reverse-engineering developer link: handoff file must be a regular file with no symlink path components ({cause})."),
        E::ArtifactStale{path}=>format!("Cannot record reverse-engineering developer link: {path} was not written in the current stage attempt."),
        E::ArtifactNotRewritten{path}=>format!("Cannot record reverse-engineering developer link: {path} was not rewritten after its prior pipeline receipt."),
        error=>error.to_string(),
    }
}

pub(super) fn current(layout: &Layout) -> Option<PipelineHandoff> {
    let expected = layout
        .record_dir()?
        .join("inception/reverse-engineering/developer-scan.md");
    let relative = expected.strip_prefix(layout.project_dir()).ok()?.to_str()?;
    observe(layout.project_dir(), &expected, relative)
        .ok()
        .flatten()
}
/// pipeline link の不足を告げる拒否文言 (upstream `aidlc-orchestrate.ts:7916-7928` 逐語)。
///
/// 2.8.2 は呼び方だけでなく案内文も変えている — 不足したまま報告できないので `next` の
/// 再実行を促し、却下が新しい試行を始めることを明記し、末尾を「古い受領証の再スタンプと
/// 検査の無効化の禁止」で締める。綴りは `aidlcToolInvocation` が畳まれた二段形であり
/// (`aidlc-runtime-paths.ts:155-168`)、クエリ側の綴りの規則 [`dispatcher_invocation`] で組む。
///
/// upstream が `--repo <repo>` を足すのは登録 repo を持つ pipeline のときだけである
/// (`evidence.repos.length > 0`)。native の [`CommandError::PipelineLinksMissing`] は repo を
/// 運ばない — repo 単位の pipeline を持たないので、この分岐は起こらない。
///
/// [`CommandError::PipelineLinksMissing`]: core_command_domain::orchestration::CommandError::PipelineLinksMissing
pub(super) fn missing_wording(stage: &str, missing: &str, single: bool) -> String {
    let refusal = if single {
        format!(
            "Cannot complete an isolated run of \"{stage}\" because these pipeline handoffs have not been recorded for this isolated run"
        )
    } else {
        format!(
            "Cannot present \"{stage}\" for approval because these pipeline handoffs have not been recorded for the current run"
        )
    };
    let resume = if single {
        format!(" --single --stage {stage}")
    } else {
        String::new()
    };
    let isolated = if single { " --single" } else { "" };
    format!(
        "{refusal}: {missing}. \
         Re-run `{next}{resume}` \
         and dispatch the missing pipeline links in their declared order, carrying the human's revision feedback. \
         Rejection starts a new attempt: earlier scans and receipts cannot certify this revision, even for a targeted artifact edit. \
         After each link returns, run `{link} --stage {stage} \
         --link <agent>{isolated}`. Do not re-stamp an old handoff or disable evidence checks to reopen the gate.",
        next = dispatcher_invocation("orchestrate next"),
        link = dispatcher_invocation("log link"),
    )
}

pub(super) async fn begin_single(
    layout: &Layout,
    directive: &core_query_use_case::orchestration::Directive,
) -> Result<bool, String> {
    use core_query_use_case::orchestration::Directive;
    let stage = match directive {
        Directive::RunStage(run) if run.is_single() => run.stage().as_str(),
        Directive::LoadSteering(load) if load.continue_token().is_single() => load.stage().as_str(),
        _ => return Ok(false),
    };
    let cursor = active_execution(layout)
        .map_err(|e| e.to_string())?
        .ok_or("single pipeline execution unavailable")?;
    let store = store_path(layout)?;
    core_command_use_case::orchestration::BeginSingleStageRunUseCase::new(
        IntentExecutionRepositoryImpl::open(&store).map_err(|e| e.to_string())?,
        IntentRepositoryImpl::open(&store).map_err(|e| e.to_string())?,
    )
    .execute(
        cursor.execution_id(),
        &core_command_domain::workflow_definition::StageSlug::parse(stage)
            .map_err(|e| e.to_string())?,
        chrono::Utc::now(),
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(true)
}
