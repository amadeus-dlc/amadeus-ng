//! `PlanFingerprintRow` — `read_plan_fingerprint` の 1 行 (実行・対象ごとの計画指紋)。

use crate::orchestration::GlobalSeqNr;

/// `read_plan_fingerprint` の 1 行。拒否も明示して古い成功を残さない
/// (指紋か拒否理由のどちらかを持つ)。
///
/// 行は値を運ぶだけである。履歴と承認入力から行を組む投影は
/// [`crate::read_tables::PlanFingerprintTables::project`] が持つ。
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
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        id: String,
        execution_id: String,
        target_id: String,
        fingerprint: Option<String>,
        error: Option<String>,
        source_digest: String,
        as_of: GlobalSeqNr,
    ) -> Self {
        Self {
            id,
            execution_id,
            target_id,
            fingerprint,
            error,
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
