//! `ReportResultRow` — 保存された報告の結果の 1 行。

/// 呼出側の報告識別子で一意に引く行。現在状態からの導出は行わない。
///
/// 行は値を運ぶだけである。材料から行を組む投影は
/// [`crate::read_tables::ReadTables::project`] が持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportResultRow {
    report_id: String,
    execution_id: String,
    stage: String,
    scope: String,
    result_kind: String,
    steps: String,
    no_op_reason: Option<String>,
    current_stage: Option<String>,
}

impl ReportResultRow {
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    #[allow(
        clippy::too_many_arguments,
        reason = "表の 1 行の全列を唯一の構築口へ渡す — 列と引数の対応を一覧で読めることを優先する"
    )]
    pub const fn new(
        report_id: String,
        execution_id: String,
        stage: String,
        scope: String,
        result_kind: String,
        steps: String,
        no_op_reason: Option<String>,
        current_stage: Option<String>,
    ) -> Self {
        Self {
            report_id,
            execution_id,
            stage,
            scope,
            result_kind,
            steps,
            no_op_reason,
            current_stage,
        }
    }

    /// SQL列へ渡すreport_id。
    #[must_use]
    pub fn report_id(&self) -> &str {
        &self.report_id
    }

    /// SQL列へ渡すexecution_id。
    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// SQL列へ渡すstage。
    #[must_use]
    pub fn stage(&self) -> &str {
        &self.stage
    }

    /// SQL列へ渡すscope。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// SQL列へ渡すresult_kind。
    #[must_use]
    pub fn result_kind(&self) -> &str {
        &self.result_kind
    }

    /// SQL列へ渡すsteps。
    #[must_use]
    pub fn steps(&self) -> &str {
        &self.steps
    }

    /// SQL列へ渡すno_op_reason。
    #[must_use]
    pub fn no_op_reason(&self) -> Option<&str> {
        self.no_op_reason.as_deref()
    }

    /// SQL列へ渡すcurrent_stage。
    #[must_use]
    pub fn current_stage(&self) -> Option<&str> {
        self.current_stage.as_deref()
    }
}
