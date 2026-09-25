//! 通常ClaudeのTaskUpdateを既存更新・投影へ接続する。
use super::{Completion, Layout, Utc};
use core_command_domain::workflow_definition::StageSlug;
use core_command_interface_adapter::orchestration::IntentExecutionRepositoryImpl;
use core_command_use_case::orchestration::SynchronizeTaskUseCase;

pub(super) async fn run(layout: &Layout, input: &str) -> Completion {
    let Some(envelope) = harness_claude::TaskUpdateEnvelope::parse(input) else {
        return Completion::silent();
    };
    if layout.state_file().is_none_or(|path| !path.exists()) {
        return Completion::silent();
    }
    let health = super::observe_hook_health(layout, "sync-workflow-state").await;
    if health.code() != 0 {
        return health;
    }
    let Some(cursor) = super::active_execution(layout).ok().flatten() else {
        return Completion::silent();
    };
    let result = async {
        let store = super::store_path(layout)?;
        let repository = IntentExecutionRepositoryImpl::open(&store).map_err(|e| e.to_string())?;
        let stage = StageSlug::parse(envelope.stage()).map_err(|e| e.to_string())?;
        SynchronizeTaskUseCase::new(repository)
            .execute(cursor.execution_id(), &stage, Utc::now())
            .await
            .map_err(|e| e.to_string())?;
        super::update_read_models(layout).await
    }
    .await;
    if let Err(error) = result {
        let args = [
            "set-status".to_string(),
            "--stage".to_string(),
            envelope.stage().to_string(),
            "--project-dir".to_string(),
            layout.project_dir().to_string_lossy().into_owned(),
        ];
        let _ =
            super::log_failure::finish(layout, "aidlc-utility", &args, Completion::refused(error))
                .await;
    }
    Completion::silent()
}
