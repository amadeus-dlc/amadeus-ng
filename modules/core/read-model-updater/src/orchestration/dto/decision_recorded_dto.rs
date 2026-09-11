//! 質問提示の永続化表現。ドメインはこの形式に依存しない。
use super::dto_decode_error::DtoDecodeError;
use core_command_domain::orchestration::{
    DecisionPrompt, DecisionRecorded, IntentExecutionEventId, IntentExecutionId,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionRecordedDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    plan_approval: Option<Box<super::plan_decision_evidence_dto::PlanDecisionEvidenceDto>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    summary_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    human_before: Option<String>,
    id: String,
    aggregate_id: String,
    stage: String,
    decision: String,
    options: Option<String>,
    rationale: Option<String>,
}
impl DecisionRecordedDto {
    pub(super) fn of(event: &DecisionRecorded) -> Self {
        let prompt = event.prompt();
        Self {
            plan_approval: prompt.plan_approval().map(|value| {
                Box::new(super::plan_decision_evidence_dto::PlanDecisionEvidenceDto::of(value))
            }),
            summary_file: prompt.summary_file().map(str::to_string),
            human_before: event.human_before().map(|id| id.as_str().to_string()),
            id: event.id().as_str().to_string(),
            aggregate_id: event.aggregate_id().as_str().to_string(),
            stage: prompt.stage().to_string(),
            decision: prompt.decision().to_string(),
            options: prompt.options().map(str::to_string),
            rationale: prompt.rationale().map(str::to_string),
        }
    }
    pub(super) fn to_domain(&self) -> Result<DecisionRecorded, DtoDecodeError> {
        if self.plan_approval.is_some() && self.summary_file.is_some() {
            return Err(DtoDecodeError::InvariantViolation);
        }
        let mut prompt = DecisionPrompt::new(&self.stage, &self.decision);
        if let Some(plan) = &self.plan_approval {
            prompt = prompt.with_plan_approval(plan.to_domain()?);
        }
        if let Some(options) = &self.options {
            prompt = prompt.with_options(options);
        }
        if let Some(rationale) = &self.rationale {
            prompt = prompt.with_rationale(rationale);
        }
        if let Some(file) = &self.summary_file {
            prompt = prompt.with_summary_file(file);
        }
        Ok(DecisionRecorded::new(
            IntentExecutionEventId::parse(&self.id)
                .map_err(|_| DtoDecodeError::malformed("id", &self.id))?,
            IntentExecutionId::parse(&self.aggregate_id)
                .map_err(|_| DtoDecodeError::malformed("aggregate_id", &self.aggregate_id))?,
            prompt,
        )
        .with_human_before(
            self.human_before
                .as_ref()
                .map(|id| {
                    IntentExecutionEventId::parse(id)
                        .map_err(|_| DtoDecodeError::malformed("human_before", id))
                })
                .transpose()?,
        ))
    }
}
