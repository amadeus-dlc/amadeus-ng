//! `state reuse-artifact` の構文・保存・応答の接続。
//!
//! 記録専用の受領である — 状態・カーソル・進捗は動かさず、`ARTIFACT_REUSED` の監査行を
//! 1 件積むだけである（オーナー裁定 D12）。
//!
//! 段だけは定義グラフで引く。そのために定義のリポジトリも開いて集約へ渡す — 配布実装
//! （`.claude/tools/aidlc-state.ts` の `handleReuseArtifact` が `findStageBySlug` を通す）
//! と同じく、定義に無い段の受領を監査へ残さないためである。
use super::{Completion, Layout, active_execution, after_projection, store_path};
use crate::cli::ReuseArtifactArgs;
use core_command_domain::orchestration::{ArtifactReuseError, ArtifactReuseReceipt};
use core_command_interface_adapter::orchestration::{
    IntentExecutionRepositoryImpl, IntentRepositoryImpl, WorkflowDefinitionRepositoryImpl,
};
use core_command_use_case::orchestration::{ArtifactReuseCommandError, RecordArtifactReuseUseCase};
use core_infrastructure::canon_json::{JsonValue, ObjectMembers, SerializationProfile, serialize};

pub(super) async fn run(layout: &Layout, args: &ReuseArtifactArgs) -> Completion {
    if let Some(error) = args.parse_error() {
        return Completion::refused(error.to_string());
    }
    let Some(slug) = args.slug().filter(|slug| !slug.is_empty()) else {
        return Completion::refused(crate::wording::REUSE_ARTIFACT_USAGE.to_string());
    };
    let Some(decision) = args.value("decision").filter(|value| !value.is_empty()) else {
        return Completion::refused(crate::wording::REUSE_ARTIFACT_REQUIRES_DECISION.to_string());
    };
    let Some(artifacts) = args.value("artifacts").filter(|value| !value.is_empty()) else {
        return Completion::refused(crate::wording::REUSE_ARTIFACT_REQUIRES_ARTIFACTS.to_string());
    };
    let repo = args.value("repo").filter(|repo| !repo.is_empty());
    let single = args.value("single") == Some("true");
    let receipt = match ArtifactReuseReceipt::new(
        slug.to_string(),
        decision.to_string(),
        artifacts.to_string(),
        repo.map(str::to_string),
        single,
    ) {
        Ok(receipt) => receipt,
        Err(error) => return Completion::refused(reason(&error)),
    };
    let cursor = match active_execution(layout) {
        Ok(Some(cursor)) => cursor,
        Ok(None) => {
            return Completion::refused(
                "Cannot resolve the active intent for artifact reuse logging.".into(),
            );
        }
        Err(error) => return Completion::refused(error.to_string()),
    };
    let store = match store_path(layout) {
        Ok(store) => store,
        Err(error) => return Completion::refused(error),
    };
    let (Ok(executions), Ok(intents), Ok(definitions)) = (
        IntentExecutionRepositoryImpl::open(&store),
        IntentRepositoryImpl::open(&store),
        WorkflowDefinitionRepositoryImpl::open(&store),
    ) else {
        return Completion::refused("Cannot open the artifact reuse repositories.".into());
    };
    if let Err(error) = RecordArtifactReuseUseCase::new(executions, intents, definitions)
        .execute(cursor.execution_id(), &receipt, chrono::Utc::now())
        .await
    {
        return Completion::refused(match error {
            ArtifactReuseCommandError::Rejected(error) => reason(&error),
            error => error.to_string(),
        });
    }
    after_projection(layout, || {
        let mut fields = ObjectMembers::new();
        fields.insert("emitted", JsonValue::String("ARTIFACT_REUSED".into()));
        fields.insert("slug", JsonValue::String(slug.into()));
        fields.insert("decision", JsonValue::String(decision.into()));
        fields.insert("artifacts", JsonValue::String(artifacts.into()));
        if let Some(repo) = repo {
            fields.insert("repo", JsonValue::String(repo.into()));
        }
        if single {
            fields.insert("single", JsonValue::Bool(true));
        }
        Completion::emitted(serialize(
            &JsonValue::Object(fields),
            SerializationProfile::ContractCompact,
        ))
    })
    .await
}

/// 受領拒否の理由を利用者が読む文字列へ組む（文言は出す側が持つ）。
fn reason(error: &ArtifactReuseError) -> String {
    match error {
        ArtifactReuseError::StageRequired => crate::wording::REUSE_ARTIFACT_USAGE.to_string(),
        ArtifactReuseError::ArtifactsRequired => {
            crate::wording::REUSE_ARTIFACT_REQUIRES_ARTIFACTS.to_string()
        }
        ArtifactReuseError::InvalidDecision { given } => {
            crate::wording::invalid_reuse_decision(given)
        }
        ArtifactReuseError::UnknownStage { slug } => crate::wording::state_unknown_stage(slug),
        ArtifactReuseError::Command(error) => format!("{error:?}"),
    }
}
