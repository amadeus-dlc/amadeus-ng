//! `PlanFingerprintTables` — 対象ごとの計画指紋の参照投影 (`read_plan_fingerprint`)。

use super::ReadTablesError;
use crate::orchestration::{GlobalSeqNr, JournalBatch, PlanFingerprintRow};
use core_command_domain::orchestration::{IntentExecutionId, PlanApprovalInput};
use core_infrastructure::canon_json::{JsonValue, ObjectMembers, hash_compact};

/// 1 回の投影で作った `read_plan_fingerprint` の行 (1 つの実行・対象につき 1 行)。
///
/// 行の型 ([`PlanFingerprintRow`]) は表の DAO が運ぶ値であり、RMU のポートに置く。材料から
/// 行を組む投影はこの型が持つ — [`super::TestingTables`] / [`super::SteeringTables`] と同じく、
/// 投影単位ごとの結果型が自分の構築規則を所有する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanFingerprintTables {
    row: PlanFingerprintRow,
}

impl PlanFingerprintTables {
    /// 履歴から集約を再生し、同じドメイン判断を呼ぶ (**この型の唯一の構築経路**)。
    ///
    /// # Errors
    ///
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
            row: PlanFingerprintRow::new(
                id,
                execution_id.as_str().to_string(),
                target_id,
                fingerprint,
                error,
                source_digest,
                history.scanned_to().unwrap_or(GlobalSeqNr::ZERO),
            ),
        })
    }

    /// 投影した行。
    #[must_use]
    pub const fn row(&self) -> &PlanFingerprintRow {
        &self.row
    }
}
