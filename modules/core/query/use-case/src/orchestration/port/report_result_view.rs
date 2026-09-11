//! 報告時の結果。ドメイン型や現在状態へ依存しない。
/// 報告結果表の一行の写し。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportResultView {
    execution_id: String,
    stage: String,
    scope: String,
    result_kind: String,
    steps: String,
    no_op_reason: Option<String>,
    current_stage: Option<String>,
}
impl ReportResultView {
    /// DAOが取得した一行を束ねる。
    #[must_use]
    pub const fn new(
        execution_id: String,
        stage: String,
        scope: String,
        result_kind: String,
        steps: String,
        no_op_reason: Option<String>,
        current_stage: Option<String>,
    ) -> Self {
        Self {
            execution_id,
            stage,
            scope,
            result_kind,
            steps,
            no_op_reason,
            current_stage,
        }
    }
    /// 投影済みのexecution_id。
    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }
    /// 投影済みのstage。
    #[must_use]
    pub fn stage(&self) -> &str {
        &self.stage
    }
    /// 投影済みのscope。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }
    /// 投影済みのresult_kind。
    #[must_use]
    pub fn result_kind(&self) -> &str {
        &self.result_kind
    }
    /// 投影済みのsteps。
    #[must_use]
    pub fn steps(&self) -> &str {
        &self.steps
    }
    /// 投影済みのno_op_reason。
    #[must_use]
    pub fn no_op_reason(&self) -> Option<&str> {
        self.no_op_reason.as_deref()
    }
    /// 投影済みのcurrent_stage。
    #[must_use]
    pub fn current_stage(&self) -> Option<&str> {
        self.current_stage.as_deref()
    }
}
