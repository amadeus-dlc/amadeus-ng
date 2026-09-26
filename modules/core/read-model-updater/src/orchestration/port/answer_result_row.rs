//! `AnswerResultRow` — 受理された回答の結果の 1 行。

/// 呼出側の回答IDに対応する投影結果。
///
/// 行は値を運ぶだけである。材料から行を組む投影は
/// [`crate::read_tables::ReadTables::project`] が持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerResultRow {
    answer_id: String,
    execution_id: String,
    stage: String,
    disposition: String,
}

impl AnswerResultRow {
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        answer_id: String,
        execution_id: String,
        stage: String,
        disposition: String,
    ) -> Self {
        Self {
            answer_id,
            execution_id,
            stage,
            disposition,
        }
    }

    /// 回答ID。
    #[must_use]
    pub fn answer_id(&self) -> &str {
        &self.answer_id
    }

    /// 実行ID。
    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// 対象ステージ。
    #[must_use]
    pub fn stage(&self) -> &str {
        &self.stage
    }

    /// 受理時点の扱い。
    #[must_use]
    pub fn disposition(&self) -> &str {
        &self.disposition
    }
}
