//! `CodeGenerationApprovalRow` — `read_code_generation_approval` の 1 行 (実行・対象ごとの開始可否)。

use crate::orchestration::GlobalSeqNr;

/// `read_code_generation_approval` の 1 行 — 1 つの実行・対象に対する開始可否。拒否理由も
/// 明示して古い成功を残さない。
///
/// 行は値を運ぶだけである。履歴・承認入力・受領から行を組む投影は
/// [`crate::read_tables::CodeGenerationApprovalTables::project`] が持つ。
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
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    #[allow(
        clippy::too_many_arguments,
        reason = "表の 1 行の全列を唯一の構築口へ渡す — 列と引数の対応を一覧で読めることを優先する"
    )]
    pub const fn new(
        id: String,
        execution_id: String,
        target_id: String,
        ok: bool,
        reason: String,
        unit: Option<String>,
        contract_hash: Option<String>,
        source_digest: String,
        as_of: GlobalSeqNr,
    ) -> Self {
        Self {
            id,
            execution_id,
            target_id,
            ok,
            reason,
            unit,
            contract_hash,
            source_digest,
            as_of,
        }
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
