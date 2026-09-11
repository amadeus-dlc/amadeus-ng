//! 報告を受理した事実。結果と遷移は同じイベントに属する。
use crate::orchestration::{
    IntentExecutionEventId, IntentExecutionId, ReportId, ReportResult, ReportResultError,
};

/// 呼出側の報告識別子に対応する受理結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reported {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    report_id: ReportId,
    result: ReportResult,
    validation: Option<crate::orchestration::StageValidation>,
    source_baseline: Option<crate::orchestration::SourceBaseline>,
}
impl Reported {
    /// イベント復号境界でも使う完全コンストラクタ。
    /// # Errors
    /// 操作列と遷移事実が一致しない場合。
    pub fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        report_id: ReportId,
        result: ReportResult,
        validation: Option<crate::orchestration::StageValidation>,
        source_baseline: Option<crate::orchestration::SourceBaseline>,
    ) -> Result<Self, ReportResultError> {
        if let ReportResult::Committed {
            steps, transition, ..
        } = &result
            && !transition.accepts_steps(steps)
        {
            return Err(ReportResultError::TransitionMismatch);
        }
        if source_baseline.is_some()
            && !matches!(
                &result,
                ReportResult::Committed {
                    transition: crate::orchestration::ReportTransition::GateApproved { .. }
                        | crate::orchestration::ReportTransition::StageSkipped { .. },
                    ..
                }
            )
        {
            return Err(ReportResultError::BaselineWithoutAdvance);
        }
        Ok(Self {
            id,
            aggregate_id,
            report_id,
            result,
            validation,
            source_baseline,
        })
    }
    /// 次の工程開始へ束縛された採取済みソース。
    #[must_use]
    pub const fn source_baseline(&self) -> Option<&crate::orchestration::SourceBaseline> {
        self.source_baseline.as_ref()
    }
    /// 完了時に採取した検証根拠。
    #[must_use]
    pub const fn validation(&self) -> Option<&crate::orchestration::StageValidation> {
        self.validation.as_ref()
    }
    /// 永続化境界へイベント識別子を渡す。
    #[must_use]
    pub const fn id(&self) -> &IntentExecutionEventId {
        &self.id
    }
    /// 永続化境界へ対象の識別子を渡す。
    #[must_use]
    pub const fn aggregate_id(&self) -> &IntentExecutionId {
        &self.aggregate_id
    }
    /// 結果投影に使う呼出側の識別子。
    #[must_use]
    pub const fn report_id(&self) -> &ReportId {
        &self.report_id
    }
    /// 結果を投影するための受理事実。
    #[must_use]
    pub const fn result(&self) -> &ReportResult {
        &self.result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::{ReportNoOp, SourceBaseline};
    use crate::workflow_definition::StageSlug;

    #[test]
    fn a_no_op_cannot_claim_the_source_baseline_of_a_new_stage() {
        let result = Reported::new(
            IntentExecutionEventId::generate(),
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
            ReportId::generate(),
            ReportResult::NoOp {
                scope: "bugfix".into(),
                no_op: ReportNoOp::AlreadyAwaiting {
                    stage: StageSlug::parse("code-generation").unwrap(),
                },
            },
            None,
            Some(SourceBaseline::new(Some(String::new())).unwrap()),
        );
        assert!(
            result.is_err(),
            "no-opには新たな工程開始の観測を付けられない"
        );
    }
}
