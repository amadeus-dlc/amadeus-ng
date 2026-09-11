//! 計画回答の検証材料を写す、この側が所有する保存形式。
use super::{
    DtoDecodeError,
    plan_approval_evidence_dto::PlanApprovalEvidenceDto,
    plan_approval_runtime_dto::{choice, session},
};
use core_command_domain::orchestration::{
    IntentExecutionId, PlanAnswerInput, PlanApprovalOrigin, PlanDecisionEvidence,
};
use core_command_domain::workspace::SpaceName;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct PlanAnswerInputDto {
    space: String,
    execution_id: String,
    stage: String,
    evidence: PlanApprovalEvidenceDto,
    session: String,
    choice: String,
    source: Option<String>,
}
impl PlanAnswerInputDto {
    pub(super) fn of(value: &PlanAnswerInput) -> Self {
        Self {
            space: value.origin().space().as_str().to_string(),
            execution_id: value.origin().execution_id().to_string(),
            stage: value.stage().to_string(),
            evidence: PlanApprovalEvidenceDto::of(value.decision().evidence()),
            session: value.decision().session().raw().to_string(),
            choice: value.choice().as_str().to_string(),
            source: value.source().map(str::to_string),
        }
    }
    pub(super) fn to_domain(&self) -> Result<PlanAnswerInput, DtoDecodeError> {
        Ok(PlanAnswerInput::new(
            PlanApprovalOrigin::new(
                SpaceName::parse(&self.space)
                    .map_err(|_| DtoDecodeError::malformed("space", &self.space))?,
                IntentExecutionId::parse(&self.execution_id)
                    .map_err(|_| DtoDecodeError::malformed("execution_id", &self.execution_id))?,
            ),
            self.stage.clone(),
            PlanDecisionEvidence::new(self.evidence.to_domain()?, session(&self.session)?),
            choice(&self.choice)?,
            self.source.clone(),
        ))
    }
}
