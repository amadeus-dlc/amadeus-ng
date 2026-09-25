//! ワークスペース全体の承認更新を、複数ストアの間で直列化する呼出境界。
use super::Layout;
use chrono::Utc;
use core_command_domain::orchestration::{
    DirectivePublication, IntentExecutionId, PlanApprovalOperationId, PlanInvalidation,
};
use core_command_domain::workspace::{SpaceName, StorePath};
use core_command_interface_adapter::orchestration::{
    IntentExecutionRepositoryImpl, PlanApprovalRuntimeRepositoryImpl,
};
use core_command_use_case::orchestration::{
    EnsurePlanApprovalRuntimeUseCase, IssueDirectiveUseCase, PreparePlanInvalidationUseCase,
    RecoverPlanInvalidationUseCase,
};
use core_infrastructure::ExclusiveFileLock;
use core_query_interface_adapter::ReadModelDaos;
use core_query_use_case::orchestration::PlanApprovalOperationUseCase;
use core_read_model_updater::orchestration::{
    PlanApprovalJournalReaderImpl, PlanApprovalReadModelUpdater, ReadModelUpdater,
};
use std::path::PathBuf;
/// 排他を保持し、先行操作を回復してから新しい承認更新を行う。
#[derive(Debug)]
pub(super) struct PlanApprovalAccess {
    root: PathBuf,
    _lock: ExclusiveFileLock,
}
impl PlanApprovalAccess {
    pub(super) async fn open(layout: &Layout) -> Result<Self, String> {
        let root = layout.aidlc_root();
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join(".aidlc-runtime.lock"))
            .map_err(|error| error.to_string())?;
        let lock = ExclusiveFileLock::acquire(file, std::time::Duration::from_secs(5))
            .map_err(|error| error.to_string())?;
        let mut access = Self { root, _lock: lock };
        access.initialize().await?;
        access.recover().await?;
        Ok(access)
    }
    async fn initialize(&mut self) -> Result<(), String> {
        let state_path = self.root.join(".aidlc-runtime.state.json");
        let store_path = StorePath::for_runtime(&self.root);
        let phase = prepare_shared_store(&self.root)?;
        match phase {
            InitializationPhase::Ready => {
                if !store_path.as_path().is_file() {
                    return Err("Previously used shared approval store is missing".to_string());
                }
            }
            InitializationPhase::Initializing => {
                EnsurePlanApprovalRuntimeUseCase::new(self.repository()?)
                    .execute(Utc::now())
                    .await
                    .map_err(|error| error.to_string())?;
                core_infrastructure::atomic::write_file_atomic(
                    &state_path,
                    b"{\"version\":1,\"phase\":\"ready\"}\n",
                )
                .map_err(|error| error.to_string())?;
            }
        }
        Ok(())
    }
    fn repository(&self) -> Result<PlanApprovalRuntimeRepositoryImpl, String> {
        PlanApprovalRuntimeRepositoryImpl::open(&StorePath::for_runtime(&self.root))
            .map_err(|error| error.to_string())
    }
    async fn project(&mut self) -> Result<(), String> {
        let reader = PlanApprovalJournalReaderImpl::open(&StorePath::for_runtime(&self.root))
            .map_err(|error| error.to_string())?;
        PlanApprovalReadModelUpdater::new(reader)
            .update_read_models()
            .await
            .map_err(|error| error.to_string())
    }
    async fn recover(&mut self) -> Result<(), String> {
        self.project().await?;
        let daos = ReadModelDaos::open(StorePath::for_runtime(&self.root).as_path())
            .map_err(|error| error.to_string())?;
        let pending = PlanApprovalOperationUseCase::new(daos.plan_approval_operation())
            .pending()
            .map_err(|error| error.to_string())?;
        for row in pending {
            let operation =
                PlanApprovalOperationId::parse(row.id()).map_err(|error| error.to_string())?;
            if row.kind() == "generation" {
                self.recover_generation(&operation).await?;
                continue;
            }
            let space = SpaceName::parse(
                row.space()
                    .ok_or("Pending approval operation has no source space")?,
            )
            .map_err(|_| "Pending approval operation has invalid source space".to_string())?;
            let execution = IntentExecutionId::parse(
                row.execution_id()
                    .ok_or("Pending approval operation has no source execution")?,
            )
            .map_err(|error| error.to_string())?;
            match row.kind() {
                "publication" => self.resolve(&operation, &space, &execution).await?,
                "answer" => self.recover_answer(&operation, &space, &execution).await?,
                "response" => {
                    self.recover_response(&operation, &space, &execution)
                        .await?
                }
                kind => return Err(format!("Unknown pending approval operation kind: {kind}")),
            }
        }
        self.project().await
    }
    async fn recover_generation(
        &mut self,
        operation: &PlanApprovalOperationId,
    ) -> Result<(), String> {
        let project = self
            .root
            .parent()
            .ok_or("Approval workspace has no project root")?;
        let source = crate::source_fingerprint::read(project).ok();
        core_command_use_case::orchestration::CertifyGenerationUseCase::new(self.repository()?)
            .execute(operation, source.as_deref(), Utc::now())
            .await
            .map_err(|error| error.to_string())
    }
    pub(super) async fn begin(
        &mut self,
        layout: &Layout,
        id: PlanApprovalOperationId,
        execution: &IntentExecutionId,
        input: &core_command_domain::orchestration::PlanApprovalInput,
    ) -> Result<(), String> {
        use core_command_interface_adapter::orchestration::IntentRepositoryImpl;
        let space = SpaceName::parse(layout.space())
            .map_err(|_| crate::wording::invalid_active_space(layout.space()))?;
        let source_path = StorePath::for_space(&self.root, &space);
        let execution_repository =
            IntentExecutionRepositoryImpl::open(&source_path).map_err(|error| error.to_string())?;
        let intent_repository =
            IntentRepositoryImpl::open(&source_path).map_err(|error| error.to_string())?;
        let source_before = crate::source_fingerprint::read(layout.project_dir()).ok();
        core_command_use_case::orchestration::BeginGenerationUseCase::new(
            self.repository()?,
            execution_repository,
            intent_repository,
        )
        .execute(id, execution, input, source_before.as_deref(), Utc::now())
        .await
        .map_err(|error| error.to_string())?;
        // 回復ループが候補を投影してからソースを再観測する。開始済みの要求は候補にならない。
        self.recover().await
    }
    fn origin_layout(
        &self,
        space: &SpaceName,
        execution: &IntentExecutionId,
    ) -> Result<Layout, String> {
        use core_query_use_case::orchestration::{FindExecutionUseCase, FindIntentRecordUseCase};
        let store = StorePath::for_space(&self.root, space);
        let daos = ReadModelDaos::open(store.as_path()).map_err(|error| error.to_string())?;
        let view = FindExecutionUseCase::new(daos.execution())
            .execute(execution.as_str())
            .map_err(|error| error.to_string())?
            .ok_or("Source execution projection is missing")?;
        let registry = self
            .root
            .join("spaces")
            .join(space.as_str())
            .join("intents/intents.json");
        let record = FindIntentRecordUseCase::new(
            core_query_interface_adapter::IntentRecordDaoImpl::new(registry),
        )
        .execute(view.intent_id())
        .map_err(|error| error.to_string())?
        .ok_or("Source intent record is missing")?;
        let directory = core_command_domain::workspace::IntentDirName::parse(record.directory())
            .map_err(|_| "Invalid source intent record directory".to_string())?;
        let project = self
            .root
            .parent()
            .ok_or("Approval workspace has no project root")?;
        let layout = Layout::for_record(project, space, &directory);
        let cursor = super::active_execution(&layout)
            .map_err(|error| error.to_string())?
            .ok_or("Source execution cursor is missing")?;
        if cursor.execution_id() != execution {
            return Err("Source record now points at another execution".to_string());
        }
        Ok(layout)
    }
    async fn project_origin(
        &mut self,
        space: &SpaceName,
        execution: &IntentExecutionId,
    ) -> Result<(), String> {
        let layout = self.origin_layout(space, execution)?;
        super::update_read_models(&layout).await
    }
    async fn recover_response(
        &mut self,
        operation: &PlanApprovalOperationId,
        space: &SpaceName,
        execution: &IntentExecutionId,
    ) -> Result<(), String> {
        let source_path = StorePath::for_space(&self.root, space);
        if !source_path.as_path().is_file() {
            return Err("Pending human response source store is missing".to_string());
        }
        let source =
            IntentExecutionRepositoryImpl::open(&source_path).map_err(|error| error.to_string())?;
        core_command_use_case::orchestration::RecordPreparedPlanResponseUseCase::new(
            self.repository()?,
            source,
        )
        .execute(operation, space, execution, Utc::now())
        .await
        .map_err(|error| error.to_string())?;
        self.project_origin(space, execution).await?;
        let source =
            IntentExecutionRepositoryImpl::open(&source_path).map_err(|error| error.to_string())?;
        core_command_use_case::orchestration::CompletePlanResponseUseCase::new(
            self.repository()?,
            source,
        )
        .execute(operation, space, execution, Utc::now())
        .await
        .map_err(|error| error.to_string())
    }
    async fn recover_answer(
        &mut self,
        operation: &PlanApprovalOperationId,
        space: &SpaceName,
        execution: &IntentExecutionId,
    ) -> Result<(), String> {
        use core_command_use_case::orchestration::{
            CompletePlanAnswerUseCase, DeliverPlanAnswerUseCase,
        };
        let source_path = StorePath::for_space(&self.root, space);
        if !source_path.as_path().is_file() {
            return Err("Pending plan answer source store is missing".to_string());
        }
        let project = self
            .root
            .parent()
            .ok_or("Approval workspace has no project root")?;
        let source_hash = crate::source_fingerprint::read(project).ok();
        let source =
            IntentExecutionRepositoryImpl::open(&source_path).map_err(|error| error.to_string())?;
        let delivered = DeliverPlanAnswerUseCase::new(self.repository()?, source)
            .execute(
                operation,
                space,
                execution,
                source_hash.as_deref(),
                Utc::now(),
            )
            .await;
        if let Err(error) = delivered {
            self.project().await?;
            return Err(error.to_string());
        }
        self.project_origin(space, execution).await?;
        let source =
            IntentExecutionRepositoryImpl::open(&source_path).map_err(|error| error.to_string())?;
        CompletePlanAnswerUseCase::new(self.repository()?, source)
            .execute(operation, space, execution, Utc::now())
            .await
            .map_err(|error| error.to_string())
    }
    async fn record_answer(
        &mut self,
        request: &core_command_use_case::orchestration::PlanAnswerRequest,
    ) -> Result<(), String> {
        use core_command_interface_adapter::orchestration::IntentRepositoryImpl;
        use core_command_use_case::orchestration::RecordPlanAnswerUseCase;
        let source_path = StorePath::for_space(&self.root, request.origin().space());
        let executions =
            IntentExecutionRepositoryImpl::open(&source_path).map_err(|error| error.to_string())?;
        let intents =
            IntentRepositoryImpl::open(&source_path).map_err(|error| error.to_string())?;
        RecordPlanAnswerUseCase::new(self.repository()?, executions, intents)
            .execute(request, Utc::now())
            .await
            .map_err(|error| error.to_string())?;
        self.project().await?;
        let delivered = self
            .recover_answer(
                request.operation_id(),
                request.origin().space(),
                request.origin().execution_id(),
            )
            .await;
        let projection = self.project().await;
        delivered?;
        projection
    }
    pub(super) async fn record_response(
        &mut self,
        layout: &Layout,
        execution: &IntentExecutionId,
        session: core_command_domain::orchestration::PlanSession,
        response: &str,
    ) -> Result<(), String> {
        use core_command_domain::orchestration::{PlanApprovalOrigin, PlanRuntimeError};
        use core_command_use_case::orchestration::{
            ObservePromptUseCase, PlanApprovalCommandError, PreparePlanResponseUseCase,
        };
        let space = SpaceName::parse(layout.space())
            .map_err(|_| crate::wording::invalid_active_space(layout.space()))?;
        let operation = PlanApprovalOperationId::generate();
        let preparation = PreparePlanResponseUseCase::new(self.repository()?)
            .execute(
                operation.clone(),
                PlanApprovalOrigin::new(space.clone(), execution.clone()),
                session.clone(),
                response,
                Utc::now(),
            )
            .await;
        match preparation {
            Ok(()) => {
                let delivered = self.recover_response(&operation, &space, execution).await;
                let shared_projection = self.project().await;
                delivered?;
                shared_projection
            }
            Err(error) => {
                // 提示がないセッションでも、実際の人間ターンの監査は通常どおり記録する。
                // 共有側に保存できなかった応答を、後から新しい質問へ結び付け直さない。
                let source =
                    IntentExecutionRepositoryImpl::open(&StorePath::for_space(&self.root, &space))
                        .map_err(|error| error.to_string())?;
                ObservePromptUseCase::new(source)
                    .execute(execution, session.raw(), response, false, Utc::now())
                    .await
                    .map_err(|error| error.to_string())?;
                super::update_read_models(layout).await?;
                if matches!(
                    error,
                    PlanApprovalCommandError::Domain(PlanRuntimeError::NoPendingChallenge)
                ) {
                    Ok(())
                } else {
                    Err(error.to_string())
                }
            }
        }
    }
    pub(super) async fn record_decision(
        &mut self,
        layout: &Layout,
        execution: &IntentExecutionId,
        request: &core_command_use_case::orchestration::PlanDecisionRequest,
    ) -> Result<(), String> {
        let path = super::store_path(layout)?;
        let source =
            IntentExecutionRepositoryImpl::open(&path).map_err(|error| error.to_string())?;
        let intents =
            core_command_interface_adapter::orchestration::IntentRepositoryImpl::open(&path)
                .map_err(|error| error.to_string())?;
        let result = core_command_use_case::orchestration::RecordPlanDecisionUseCase::new(
            self.repository()?,
            source,
            intents,
        )
        .execute(execution, request, Utc::now())
        .await
        .map_err(|error| error.to_string());
        // 本家と同じく、選択肢数などの後段で失敗しても保存済みの監査事実は投影する。
        let projection = super::update_read_models(layout).await;
        result?;
        projection?;
        self.project().await
    }
    async fn resolve(
        &mut self,
        operation: &PlanApprovalOperationId,
        space: &SpaceName,
        execution: &IntentExecutionId,
    ) -> Result<(), String> {
        let source_path = StorePath::for_space(&self.root, space);
        if !source_path.as_path().is_file() {
            return Err("Pending approval publication source store is missing".to_string());
        }
        let source =
            IntentExecutionRepositoryImpl::open(&source_path).map_err(|error| error.to_string())?;
        self.project_origin(space, execution).await?;
        RecoverPlanInvalidationUseCase::new(self.repository()?, source)
            .execute(operation, space, execution, Utc::now())
            .await
            .map_err(|error| error.to_string())
    }
    pub(super) async fn publish(
        &mut self,
        layout: &Layout,
        execution: &IntentExecutionId,
        publication: DirectivePublication,
    ) -> Result<(), String> {
        let space = SpaceName::parse(layout.space())
            .map_err(|_| crate::wording::invalid_active_space(layout.space()))?;
        let operation = PlanApprovalOperationId::generate();
        PreparePlanInvalidationUseCase::new(self.repository()?)
            .execute(
                PlanInvalidation::new(operation.clone(), space.clone(), execution.clone()),
                Utc::now(),
            )
            .await
            .map_err(|error| error.to_string())?;
        let source = IntentExecutionRepositoryImpl::open(&StorePath::for_space(&self.root, &space))
            .map_err(|error| error.to_string())?;
        let publication = publication.with_approval_operation(Some(operation.clone()));
        IssueDirectiveUseCase::new(source)
            .execute(execution, &publication, Utc::now())
            .await
            .map_err(|error| error.to_string())?;
        self.resolve(&operation, &space, execution).await?;
        self.project().await
    }
}

/// 初回作成の途中と、承認操作を許可した後を区別する機械ローカルの記録。
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct InitializationMarker {
    version: u32,
    phase: InitializationPhase,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum InitializationPhase {
    Initializing,
    Ready,
}

/// 検証する参照入力を読み、計画承認の質問コマンドへ渡す。
pub(super) async fn decision(
    layout: &Layout,
    args: &crate::cli::InteractionArgs,
) -> Result<(), String> {
    use core_command_domain::orchestration::{DecisionPrompt, PlanSession, PlanTarget};
    use core_command_use_case::orchestration::PlanDecisionRequest;
    use core_read_model_updater::orchestration::PlanSource;
    let target = match (
        args.value("unit"),
        args.value("stage-level") == Some("true"),
    ) {
        (Some(unit), false) => PlanTarget::for_unit(unit).map_err(|error| error.to_string())?,
        (None, true) => PlanTarget::stage_level(),
        _ => {
            return Err(
                "Plan Approval requires exactly one of --unit <unit> or --stage-level".to_string(),
            );
        }
    };
    let record = layout
        .record_dir()
        .ok_or("Code Generation approval authority requires an active workflow state")?;
    let cursor = super::active_execution(layout).map_err(|error| error.to_string())?.ok_or("Code Generation approval authority is unavailable because the active directive is missing, stale, or legacy; run a fresh `next`")?;
    let mut access = PlanApprovalAccess::open(layout).await?;
    let source = crate::source_fingerprint::read(layout.project_dir()).ok();
    let requested = crate::lexical_path::normalize(
        &layout
            .project_dir()
            .join(args.value("questions-file").unwrap_or("")),
    );
    let requested = requested
        .strip_prefix(layout.project_dir())
        .unwrap_or(&requested)
        .to_str()
        .ok_or("Plan Approval questions path is not UTF-8")?
        .replace('\\', "/");
    let input = PlanSource::new(
        layout.project_dir().to_path_buf(),
        record.to_path_buf(),
        layout.memory_dir(),
        target,
    )
    .read(source)
    .map_err(|error| error.to_string())?
    .with_supplied_questions_file(requested);
    let session = args
        .value("session")
        .map(core_infrastructure::ecmascript::trim)
        .filter(|session| !session.is_empty())
        .ok_or("Plan Approval requires --session <id> from the invoking SessionStart context.")?;
    let session = PlanSession::new(session.to_string()).map_err(|error| error.to_string())?;
    let mut prompt = DecisionPrompt::new(
        args.value("stage").unwrap_or(""),
        args.value("decision").unwrap_or(""),
    );
    if let Some(options) = args.value("options").filter(|value| !value.is_empty()) {
        prompt = prompt.with_options(options);
    }
    if let Some(rationale) = args.value("rationale").filter(|value| !value.is_empty()) {
        prompt = prompt.with_rationale(rationale);
    }
    let request = PlanDecisionRequest::new(
        PlanApprovalOperationId::generate(),
        input,
        prompt,
        session,
        args.value("exact-option-labels") == Some("true"),
    );
    access
        .record_decision(layout, cursor.execution_id(), &request)
        .await
}

/// 現在の回答文書を読み、指定操作IDの受領を保存・投影する。
pub(super) async fn answer(
    layout: &Layout,
    args: &crate::cli::InteractionArgs,
    id: PlanApprovalOperationId,
) -> Result<(), String> {
    use core_command_domain::orchestration::{
        PlanApprovalOrigin, PlanChoice, PlanSession, PlanTarget,
    };
    use core_read_model_updater::orchestration::PlanSource;
    let choice = match args.value("details") {
        Some("Approve Plan") => PlanChoice::ApprovePlan,
        Some("Request Changes") => PlanChoice::RequestChanges,
        _ => return Err("Plan Approval requires Approve Plan or Request Changes".to_string()),
    };
    let target = match (
        args.value("unit"),
        args.value("stage-level") == Some("true"),
    ) {
        (Some(unit), false) => PlanTarget::for_unit(unit).map_err(|error| error.to_string())?,
        (None, true) => PlanTarget::stage_level(),
        _ => {
            return Err(
                "Plan Approval requires exactly one of --unit <unit> or --stage-level".to_string(),
            );
        }
    };
    let record = layout
        .record_dir()
        .ok_or("Code Generation approval authority requires an active workflow state")?;
    let cursor = super::active_execution(layout).map_err(|error| error.to_string())?.ok_or("Code Generation approval authority is unavailable because the active directive is missing, stale, or legacy; run a fresh `next`")?;
    let mut access = PlanApprovalAccess::open(layout).await?;
    let source = crate::source_fingerprint::read(layout.project_dir()).ok();
    let requested = crate::lexical_path::normalize(
        &layout
            .project_dir()
            .join(args.value("questions-file").unwrap_or("")),
    );
    let requested = requested
        .strip_prefix(layout.project_dir())
        .unwrap_or(&requested)
        .to_str()
        .ok_or("Plan Approval questions path is not UTF-8")?
        .replace('\\', "/");
    let input = PlanSource::new(
        layout.project_dir().to_path_buf(),
        record.to_path_buf(),
        layout.memory_dir(),
        target,
    )
    .read(source)
    .map_err(|error| error.to_string())?
    .with_supplied_questions_file(requested);
    let session = args
        .value("session")
        .map(core_infrastructure::ecmascript::trim)
        .filter(|session| !session.is_empty())
        .ok_or("Plan Approval requires --session <id> from the invoking SessionStart context.")?;
    let session = PlanSession::new(session.to_string()).map_err(|error| error.to_string())?;

    let space = SpaceName::parse(layout.space())
        .map_err(|_| crate::wording::invalid_active_space(layout.space()))?;
    let request = core_command_use_case::orchestration::PlanAnswerRequest::new(
        id,
        input,
        PlanApprovalOrigin::new(space, cursor.execution_id().clone()),
        args.value("stage").unwrap_or("").to_string(),
        session,
        choice,
    );
    access.record_answer(&request).await
}

fn prepare_shared_store(root: &std::path::Path) -> Result<InitializationPhase, String> {
    let state_path = root.join(".aidlc-runtime.state.json");
    let store_path = StorePath::for_runtime(root);
    let phase = match std::fs::read(&state_path) {
        Ok(bytes) => {
            let marker: InitializationMarker = serde_json::from_slice(&bytes)
                .map_err(|_| "Invalid shared approval initialization marker".to_string())?;
            if marker.version != 1 {
                return Err("Unsupported shared approval initialization marker".to_string());
            }
            marker.phase
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if store_path.as_path().exists() {
                return Err("Shared approval store has no initialization marker".to_string());
            }
            core_infrastructure::atomic::write_file_atomic(
                &state_path,
                b"{\"version\":1,\"phase\":\"initializing\"}\n",
            )
            .map_err(|error| error.to_string())?;
            InitializationPhase::Initializing
        }
        Err(error) => return Err(error.to_string()),
    };
    if matches!(phase, InitializationPhase::Ready) && !store_path.as_path().is_file() {
        return Err("Previously used shared approval store is missing".to_string());
    }
    Ok(phase)
}

pub(super) fn prepare_hook_store(layout: &Layout) -> Result<(), String> {
    std::fs::create_dir_all(layout.aidlc_root()).map_err(|error| error.to_string())?;
    prepare_shared_store(&layout.aidlc_root()).map(|_| ())
}

/// 保存済み指示の文脈をPreCompactで失効させる。指示のない初回発火では承認ストアを作らない。
pub(super) async fn invalidate_context(
    layout: &Layout,
    execution: &IntentExecutionId,
    intent: &core_command_domain::orchestration::IntentId,
    session: &str,
    state: &str,
) -> Result<(), String> {
    use core_command_domain::orchestration::DirectiveContextInvalidation;
    use core_command_use_case::orchestration::InvalidateDirectiveContextUseCase;
    if session.is_empty() {
        return Ok(());
    }
    // hook-healthだけが作ったストアは承認の発行履歴を持たない。
    // 発行開始前のPreCompactを、承認状態の初期化操作に変えない。
    let state_path = layout.aidlc_root().join(".aidlc-runtime.state.json");
    let marker = match std::fs::read(&state_path) {
        Ok(bytes) => serde_json::from_slice::<InitializationMarker>(&bytes)
            .map_err(|error| error.to_string())?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.to_string()),
    };
    if marker.version != 1 {
        return Err("Unsupported shared approval initialization marker".into());
    }
    if !matches!(marker.phase, InitializationPhase::Ready) {
        return Ok(());
    }
    let project = std::fs::canonicalize(layout.project_dir()).map_err(|error| error.to_string())?;
    let project = project.to_str().ok_or("Project path is not UTF-8")?;
    let operation = PlanApprovalOperationId::generate();
    let request = DirectiveContextInvalidation::new(
        intent.clone(),
        core_infrastructure::hash::sha256_hex(project.as_bytes()),
        core_infrastructure::hash::sha256_hex(state.as_bytes()),
        session.to_string(),
    );
    let space = SpaceName::parse(layout.space())
        .map_err(|_| crate::wording::invalid_active_space(layout.space()))?;
    let source =
        IntentExecutionRepositoryImpl::open(&StorePath::for_space(&layout.aidlc_root(), &space))
            .map_err(|error| error.to_string())?;
    let mut approval = PlanApprovalAccess::open(layout).await?;
    InvalidateDirectiveContextUseCase::new(approval.repository()?, source)
        .execute(execution, &space, &operation, &request, Utc::now())
        .await
        .map_err(|error| error.to_string())?;
    super::update_read_models(layout).await?;
    approval.project().await
}
