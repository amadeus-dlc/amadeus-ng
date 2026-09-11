//! 指示発行状態の永続化表現。
use super::dto_decode_error::DtoDecodeError;
use core_command_domain::orchestration::{
    ActiveDirective, DirectivePublication, IntentId, PublishedDirective,
};
use core_command_domain::workflow_definition::StageSlug;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ActiveDirectiveDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    approval_operation_id: Option<String>,
    owner_session: String,
    owner_epoch: u64,
    context_epoch: u64,
    issuance_revision: u64,
    revision: u64,
    intent_id: String,
    project_sha256: String,
    state_sha256: String,
    initial_state_sha256: String,
    directive: PublishedDirectiveDto,
    #[serde(default)]
    source_floor: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum PublishedDirectiveDto {
    Error {
        stage: String,
    },
    RunStage {
        stage: String,
        unit: Option<String>,
    },
    LoadSteering {
        stage: String,
        part: u32,
        parts: u32,
        token: String,
    },
}
impl ActiveDirectiveDto {
    pub(super) fn of(value: &ActiveDirective) -> Self {
        Self {
            issuance_revision: value.issuance_revision(),
            owner_session: value.owner_session().to_string(),
            owner_epoch: value.owner_epoch(),
            context_epoch: value.context_epoch(),
            approval_operation_id: value.approval_operation_id().map(ToString::to_string),
            source_floor: value.source_floor().map(str::to_string),
            revision: value.revision(),
            intent_id: value.intent_id().as_str().to_string(),
            project_sha256: value.project_sha256().to_string(),
            state_sha256: value.state_sha256().to_string(),
            initial_state_sha256: value.initial_state_sha256().to_string(),
            directive: match value.directive() {
                PublishedDirective::Error { stage } => PublishedDirectiveDto::Error {
                    stage: stage.as_str().to_string(),
                },
                PublishedDirective::RunStage { stage, unit } => PublishedDirectiveDto::RunStage {
                    stage: stage.as_str().to_string(),
                    unit: unit.clone(),
                },
                PublishedDirective::LoadSteering {
                    stage,
                    part,
                    parts,
                    token,
                } => PublishedDirectiveDto::LoadSteering {
                    stage: stage.as_str().to_string(),
                    part: *part,
                    parts: *parts,
                    token: token.clone(),
                },
            },
        }
    }
    pub(super) fn to_domain(&self) -> Result<ActiveDirective, DtoDecodeError> {
        let directive = match &self.directive {
            PublishedDirectiveDto::Error { stage } => PublishedDirective::Error {
                stage: StageSlug::parse(stage)
                    .map_err(|_| DtoDecodeError::malformed("stage", stage))?,
            },
            PublishedDirectiveDto::RunStage { stage, unit } => PublishedDirective::RunStage {
                stage: StageSlug::parse(stage)
                    .map_err(|_| DtoDecodeError::malformed("stage", stage))?,
                unit: unit.clone(),
            },
            PublishedDirectiveDto::LoadSteering {
                stage,
                part,
                parts,
                token,
            } => PublishedDirective::LoadSteering {
                stage: StageSlug::parse(stage)
                    .map_err(|_| DtoDecodeError::malformed("stage", stage))?,
                part: *part,
                parts: *parts,
                token: token.clone(),
            },
        };
        Ok(ActiveDirective::new(
            self.revision,
            IntentId::parse(&self.intent_id)
                .map_err(|_| DtoDecodeError::malformed("intent_id", &self.intent_id))?,
            DirectivePublication::new(
                self.project_sha256.clone(),
                self.state_sha256.clone(),
                directive,
            )
            .with_source_floor(self.source_floor.clone())
            .with_approval_operation(
                self.approval_operation_id
                    .as_ref()
                    .map(|id| {
                        core_command_domain::orchestration::PlanApprovalOperationId::parse(id)
                            .map_err(|_| DtoDecodeError::malformed("approval_operation_id", id))
                    })
                    .transpose()?,
            ),
            self.initial_state_sha256.clone(),
            self.owner_session.clone(),
            self.owner_epoch,
            self.context_epoch,
            self.issuance_revision,
        ))
    }
}
