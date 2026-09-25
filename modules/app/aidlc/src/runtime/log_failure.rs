//! 本家2.7.1 emitErrorに対応するlog面の共通終了処理。
use super::{Completion, active_execution, store_path, update_read_models};
use crate::layout::Layout;
use chrono::Utc;
use core_command_domain::orchestration::CommandFailure;
use core_command_interface_adapter::orchestration::IntentExecutionRepositoryImpl;
use core_command_use_case::orchestration::RecordCommandFailureUseCase;
use core_infrastructure::canon_json::{JsonValue, ObjectMembers, SerializationProfile, serialize};

/// 失敗監査の保存不能でも元の拒否は必ず返す（固定本家emitErrorの契約）。
pub(super) async fn finish(
    layout: &Layout,
    tool: &str,
    args: &[String],
    completion: Completion,
) -> Completion {
    if completion.code() != 1 {
        return completion;
    }
    let Some(message) = completion.diagnostic() else {
        return completion;
    };
    // 監査値の project dir は投影の描画出口で `<project-dir>` へ伏せる
    // (`core_read_model_updater::workspace::AuditRedaction` — upstream `renderAuditBlock` と
    // 同じ場所)。ここでは生の綴りのまま事実を保存する。
    let failure = CommandFailure::new(
        tool.to_string(),
        core_infrastructure::ecmascript::trim(&format!("{tool} {}", args.join(" "))).to_string(),
        message.to_string(),
    );
    // 既存の状態がないコマンド失敗で新規intentやストアを作らない。
    if layout.state_file().is_some_and(|path| path.exists()) {
        let _ = record(layout, &failure).await;
    }
    let mut fields = ObjectMembers::new();
    fields.insert("error", JsonValue::String(message.to_string()));
    Completion::refused(serialize(
        &JsonValue::Object(fields),
        SerializationProfile::ContractCompact,
    ))
}

async fn record(layout: &Layout, failure: &CommandFailure) -> Result<(), String> {
    let cursor = active_execution(layout)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "active execution unavailable".to_string())?;
    let store = store_path(layout)?;
    let repository =
        IntentExecutionRepositoryImpl::open(&store).map_err(|error| error.to_string())?;
    RecordCommandFailureUseCase::new(repository)
        .execute(cursor.execution_id(), failure, Utc::now())
        .await
        .map_err(|error| error.to_string())?;
    update_read_models(layout).await
}
