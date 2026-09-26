//! `CodeGenerationApprovalTables` — 対象ごとの「コード生成を開始できるか」の参照投影
//! (`read_code_generation_approval`)。
//!
//! # 材料が 2 つのジャーナルにまたがる
//!
//! 判断 [`IntentExecution::code_generation_approval`] は、空間側のジャーナルが持つ集約
//! （`IntentExecution` と `Intent`）と、共有ランタイム側のジャーナルが持つ受領
//! （`PlanApprovalRuntime` の [`PlanReceipts`]）の**両方**を材料に取る。前者は
//! [`super::PlanFingerprintTables`] と同じく履歴から `replay` して起こし、後者は取得ループが
//! 共有ランタイム側の読み手から読んで渡す。投影核はどちらの読み手も知らない。
//!
//! [`IntentExecution::code_generation_approval`]: core_command_domain::orchestration::IntentExecution::code_generation_approval
use super::ReadTablesError;
use crate::orchestration::{CodeGenerationApprovalRow, GlobalSeqNr, JournalBatch};
use core_command_domain::orchestration::{IntentExecutionId, PlanApprovalInput, PlanReceipts};
use core_infrastructure::canon_json::{JsonValue, ObjectMembers, hash_compact};

/// 1 回の投影で作った `read_code_generation_approval` の行 (1 つの実行・対象につき 1 行)。
///
/// 行の型 ([`CodeGenerationApprovalRow`]) は表の DAO が運ぶ値であり、RMU のポートに置く。
/// 材料から行を組む投影はこの型が持つ — 投影単位ごとの結果型が自分の構築規則を所有する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeGenerationApprovalTables {
    row: CodeGenerationApprovalRow,
}

impl CodeGenerationApprovalTables {
    /// 履歴から集約を再生し、共有側の受領と合わせて同じドメイン判断を呼ぶ
    /// (**この型の唯一の構築経路**)。
    ///
    /// # Errors
    ///
    /// 実行または依頼を再構成できない場合。
    pub fn project(
        history: &JournalBatch,
        execution_id: &IntentExecutionId,
        input: &PlanApprovalInput,
        receipts: &PlanReceipts,
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
        let approval = execution.code_generation_approval_from_receipts(intent, input, receipts);
        let target_id = input.target().id();
        let mut key = ObjectMembers::new();
        key.insert(
            "execution_id",
            JsonValue::String(execution_id.as_str().to_string()),
        );
        key.insert("target_id", JsonValue::String(target_id.clone()));
        let id = hash_compact(&JsonValue::Object(key)).rendered();
        // 受領は鍵で数える — 同じ鍵の集合なら判断は変わらない。
        let receipt_keys = receipts
            .iter()
            .map(|receipt| receipt.key())
            .collect::<Vec<_>>()
            .join(",");
        let material = JsonValue::Array(
            [
                if approval.ok() { "ok" } else { "refused" },
                approval.reason(),
                approval.contract_hash().unwrap_or_default(),
                approval.unit().unwrap_or_default(),
                &receipt_keys,
                input.documents().plan(),
                input.documents().instructions(),
                input.documents().questions(),
                input.sections().org(),
                input.sections().team(),
                input.sections().project(),
                input.state_sha256().unwrap_or_default(),
                input.source_sha256().unwrap_or_default(),
            ]
            .into_iter()
            .map(|value| JsonValue::String(value.to_string()))
            .collect(),
        );
        let source_digest = hash_compact(&material).rendered();
        Ok(Self {
            row: CodeGenerationApprovalRow::new(
                id,
                execution_id.as_str().to_string(),
                target_id,
                approval.ok(),
                approval.reason().to_string(),
                approval.unit().map(str::to_string),
                approval.contract_hash().map(str::to_string),
                source_digest,
                history.scanned_to().unwrap_or(GlobalSeqNr::ZERO),
            ),
        })
    }

    /// 投影した行。
    #[must_use]
    pub const fn row(&self) -> &CodeGenerationApprovalRow {
        &self.row
    }
}
