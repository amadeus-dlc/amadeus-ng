//! 回答IDで取得する結果。ドメイン型に依存しない。
/// 回答の保存時点の結果View。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerResultView {
    execution_id: String,
    stage: String,
    disposition: String,
}
impl AnswerResultView {
    /// 行の値をそのまま保持する。
    #[must_use]
    pub const fn new(execution_id: String, stage: String, disposition: String) -> Self {
        Self {
            execution_id,
            stage,
            disposition,
        }
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
    /// 保存された受理結果。
    #[must_use]
    pub fn disposition(&self) -> &str {
        &self.disposition
    }
}
