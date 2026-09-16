//! 対象ごとの「コード生成を開始できるか」の参照投影。
//!
//! # 材料が 2 つのジャーナルにまたがる
//!
//! 判断 [`IntentExecution::code_generation_approval`] は、空間側のジャーナルが持つ集約
//! （`IntentExecution` と `Intent`）と、共有ランタイム側のジャーナルが持つ受領
//! （`PlanApprovalRuntime` の [`PlanReceipts`]）の**両方**を材料に取る。前者は
//! [`super::PlanFingerprintRow`] と同じく履歴から `replay` して起こし、後者は取得ループが
//! 共有ランタイム側の読み手から読んで渡す。投影核はどちらの読み手も知らない。
use super::ReadTablesError;
use crate::orchestration::{GlobalSeqNr, JournalBatch};
use core_command_domain::orchestration::{IntentExecutionId, PlanApprovalInput, PlanReceipts};
use core_infrastructure::canon_json::{JsonValue, ObjectMembers, hash_compact};

/// 1 つの実行・対象に対する開始可否。拒否理由も明示して古い成功を残さない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeGenerationApprovalRow {
    id: String,
    execution_id: String,
    target_id: String,
    ok: bool,
    reason: String,
    unit: Option<String>,
    contract_hash: Option<String>,
    source_digest: String,
    as_of: GlobalSeqNr,
}

impl CodeGenerationApprovalRow {
    /// 履歴から集約を再生し、共有側の受領と合わせて同じドメイン判断を呼ぶ。
    ///
    /// # Errors
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
            id,
            execution_id: execution_id.as_str().to_string(),
            target_id,
            ok: approval.ok(),
            reason: approval.reason().to_string(),
            unit: approval.unit().map(str::to_string),
            contract_hash: approval.contract_hash().map(str::to_string),
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

    /// 開始できるか。
    #[must_use]
    pub const fn ok(&self) -> bool {
        self.ok
    }

    /// 判断の理由（成功時も `approved` が入る）。
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// 対象 Unit。段階全体は `None`。
    #[must_use]
    pub fn unit(&self) -> Option<&str> {
        self.unit.as_deref()
    }

    /// 承認が束ねたテスト契約の自己ハッシュ。
    #[must_use]
    pub fn contract_hash(&self) -> Option<&str> {
        self.contract_hash.as_deref()
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
