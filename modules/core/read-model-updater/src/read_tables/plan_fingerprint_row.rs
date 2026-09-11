//! 対象ごとの計画指紋の参照投影。
use super::ReadTablesError;
use crate::orchestration::{GlobalSeqNr, JournalBatch};
use core_command_domain::orchestration::{IntentExecutionId, PlanApprovalInput};
use core_infrastructure::canon_json::{JsonValue, ObjectMembers, hash_compact};
/// 1つの実行・対象に対応する参照モデル。拒否も明示して古い成功を残さない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanFingerprintRow {
    id: String,
    execution_id: String,
    target_id: String,
    fingerprint: Option<String>,
    error: Option<String>,
    source_digest: String,
    as_of: GlobalSeqNr,
}
impl PlanFingerprintRow {
    /// 履歴から集約を再生し、同じドメイン判断を呼ぶ。
    /// # Errors
    /// 実行または依頼を再構成できない場合。
    pub fn project(
        history: &JournalBatch,
        execution_id: &IntentExecutionId,
        input: &PlanApprovalInput,
    ) -> Result<Self, ReadTablesError> {
        let execution = super::replay_executions(history)?
            .into_iter()
            .find(|execution| execution.id() == execution_id)
            .ok_or_else(|| ReadTablesError::IntentUnavailable {
                execution_id: execution_id.as_str().to_string(),
                intent_id: String::new(),
            })?;
        let intent = history
            .intents()
            .iter()
            .find(|intent| intent.id() == execution.intent_id())
            .ok_or_else(|| ReadTablesError::IntentUnavailable {
                execution_id: execution_id.as_str().to_string(),
                intent_id: execution.intent_id().as_str().to_string(),
            })?;
        let (fingerprint, error) = match execution.plan_fingerprint(intent, input) {
            Ok(value) => (Some(value), None),
            Err(error) => (None, Some(error.to_string())),
        };
        let target_id = input.target().id();
        let mut key = ObjectMembers::new();
        key.insert(
            "execution_id",
            JsonValue::String(execution_id.as_str().to_string()),
        );
        key.insert("target_id", JsonValue::String(target_id.clone()));
        let id = hash_compact(&JsonValue::Object(key)).rendered();
        let material = JsonValue::Array(
            [
                fingerprint.as_deref().unwrap_or_default(),
                error.as_deref().unwrap_or_default(),
                input.documents().plan(),
                input.documents().instructions(),
                input.documents().questions(),
                input.sections().org(),
                input.sections().team(),
                input.sections().project(),
                input.state_sha256().unwrap_or_default(),
            ]
            .into_iter()
            .map(|value| JsonValue::String(value.to_string()))
            .collect(),
        );
        let source_digest = hash_compact(&material).rendered();
        Ok(Self {
            id,
            execution_id: execution_id.as_str().to_string(),
            target_id,
            fingerprint,
            error,
            source_digest,
            as_of: history.scanned_to().unwrap_or(GlobalSeqNr::ZERO),
        })
    }
    /// 行の識別子。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// 実行の識別子。
    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }
    /// 対象の識別子。
    #[must_use]
    pub fn target_id(&self) -> &str {
        &self.target_id
    }
    /// 計算済みの計画指紋。
    #[must_use]
    pub fn fingerprint(&self) -> Option<&str> {
        self.fingerprint.as_deref()
    }
    /// 拒否理由。
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    /// 入力と結果の照合子。
    #[must_use]
    pub fn source_digest(&self) -> &str {
        &self.source_digest
    }
    /// 集約を取得した履歴位置。
    #[must_use]
    pub const fn as_of(&self) -> GlobalSeqNr {
        self.as_of
    }
}
