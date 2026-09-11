//! 報告事実の永続化表現。ドメインはserdeやSQLへ依存しない。
use super::dto_decode_error::DtoDecodeError;
use super::source_baseline_dto::SourceBaselineDto;
use core_command_domain::orchestration::{
    ArtifactPaths, IntentExecutionEventId, IntentExecutionId, ReportId, ReportNoOp, ReportResult,
    ReportTransition, Reported, TransitionStep, TransitionSteps,
};
use core_command_domain::workflow_definition::StageSlug;
use serde::{Deserialize, Serialize};

/// 報告イベントの保存形式。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportedDto {
    id: String,
    aggregate_id: String,
    report_id: String,
    result: ResultDto,
    source_baseline: Option<SourceBaselineDto>,
    validation_basis: Option<String>,
    validation_warning: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum ResultDto {
    Committed {
        stage: String,
        scope: String,
        steps: Vec<String>,
        transition: TransitionDto,
    },
    NoOp {
        scope: String,
        no_op: NoOpDto,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum TransitionDto {
    GateOpened { artifacts: Vec<String> },
    GateApproved { user_input: Option<String> },
    GateRejected { feedback: Option<String> },
    StageRevised,
    StageSkipped { reason: String },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum NoOpDto {
    AlreadyAwaiting { stage: String },
    AlreadyCompletedMovedOn { stage: String, current: String },
    WorkflowAlreadyCompleted { stage: String },
}
impl ReportedDto {
    pub(super) fn of(reported: &Reported) -> Self {
        let result = match reported.result() {
            ReportResult::Committed {
                stage,
                scope,
                steps,
                transition,
            } => ResultDto::Committed {
                stage: stage.to_string(),
                scope: scope.clone(),
                steps: steps.fold_left(Vec::new(), |mut values, step| {
                    values.push(format!("{step:?}"));
                    values
                }),
                transition: match transition {
                    ReportTransition::GateOpened { artifacts } => TransitionDto::GateOpened {
                        artifacts: artifacts.fold_left(Vec::new(), |mut values, path| {
                            values.push(path.to_string());
                            values
                        }),
                    },
                    ReportTransition::GateApproved { user_input } => TransitionDto::GateApproved {
                        user_input: user_input.clone(),
                    },
                    ReportTransition::GateRejected { feedback } => TransitionDto::GateRejected {
                        feedback: feedback.clone(),
                    },
                    ReportTransition::StageRevised => TransitionDto::StageRevised,
                    ReportTransition::StageSkipped { reason } => TransitionDto::StageSkipped {
                        reason: reason.clone(),
                    },
                },
            },
            ReportResult::NoOp { scope, no_op } => ResultDto::NoOp {
                scope: scope.clone(),
                no_op: match no_op {
                    ReportNoOp::AlreadyAwaiting { stage } => NoOpDto::AlreadyAwaiting {
                        stage: stage.to_string(),
                    },
                    ReportNoOp::AlreadyCompletedMovedOn { stage, current } => {
                        NoOpDto::AlreadyCompletedMovedOn {
                            stage: stage.to_string(),
                            current: current.to_string(),
                        }
                    }
                    ReportNoOp::WorkflowAlreadyCompleted { stage } => {
                        NoOpDto::WorkflowAlreadyCompleted {
                            stage: stage.to_string(),
                        }
                    }
                },
            },
        };
        Self {
            id: reported.id().to_string(),
            aggregate_id: reported.aggregate_id().to_string(),
            report_id: reported.report_id().to_string(),
            result,
            source_baseline: reported.source_baseline().map(SourceBaselineDto::of),
            validation_basis: match reported.validation() {
                Some(core_command_domain::orchestration::StageValidation::Basis(value)) => {
                    Some(value.clone())
                }
                _ => None,
            },
            validation_warning: match reported.validation() {
                Some(core_command_domain::orchestration::StageValidation::Warning(value)) => {
                    Some(value.clone())
                }
                _ => None,
            },
        }
    }
    pub(super) fn to_domain(&self) -> Result<Reported, DtoDecodeError> {
        let slug =
            |raw: &str| StageSlug::parse(raw).map_err(|_| DtoDecodeError::malformed("stage", raw));
        let result = match &self.result {
            ResultDto::NoOp { scope, no_op } => ReportResult::NoOp {
                scope: scope.clone(),
                no_op: match no_op {
                    NoOpDto::AlreadyAwaiting { stage } => ReportNoOp::AlreadyAwaiting {
                        stage: slug(stage)?,
                    },
                    NoOpDto::AlreadyCompletedMovedOn { stage, current } => {
                        ReportNoOp::AlreadyCompletedMovedOn {
                            stage: slug(stage)?,
                            current: slug(current)?,
                        }
                    }
                    NoOpDto::WorkflowAlreadyCompleted { stage } => {
                        ReportNoOp::WorkflowAlreadyCompleted {
                            stage: slug(stage)?,
                        }
                    }
                },
            },
            ResultDto::Committed {
                stage,
                scope,
                steps,
                transition,
            } => {
                let steps = TransitionSteps::new(
                    steps
                        .iter()
                        .map(|step| match step.as_str() {
                            "GateStart" => Ok(TransitionStep::GateStart),
                            "GateStartRecovered" => Ok(TransitionStep::GateStartRecovered),
                            "Approve" => Ok(TransitionStep::Approve),
                            "Reject" => Ok(TransitionStep::Reject),
                            "Revise" => Ok(TransitionStep::Revise),
                            "Skip" => Ok(TransitionStep::Skip),
                            _ => Err(DtoDecodeError::malformed("steps", step)),
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                )
                .map_err(|_| DtoDecodeError::InvariantViolation)?;
                let transition = match transition {
                    TransitionDto::GateOpened { artifacts } => ReportTransition::GateOpened {
                        artifacts: ArtifactPaths::new(artifacts.clone()),
                    },
                    TransitionDto::GateApproved { user_input } => ReportTransition::GateApproved {
                        user_input: user_input.clone(),
                    },
                    TransitionDto::GateRejected { feedback } => ReportTransition::GateRejected {
                        feedback: feedback.clone(),
                    },
                    TransitionDto::StageRevised => ReportTransition::StageRevised,
                    TransitionDto::StageSkipped { reason } => ReportTransition::StageSkipped {
                        reason: reason.clone(),
                    },
                };
                ReportResult::Committed {
                    stage: slug(stage)?,
                    scope: scope.clone(),
                    steps,
                    transition,
                }
            }
        };
        Reported::new(
            IntentExecutionEventId::parse(&self.id)
                .map_err(|_| DtoDecodeError::malformed("id", &self.id))?,
            IntentExecutionId::parse(&self.aggregate_id)
                .map_err(|_| DtoDecodeError::malformed("aggregate_id", &self.aggregate_id))?,
            ReportId::parse(&self.report_id)
                .map_err(|_| DtoDecodeError::malformed("report_id", &self.report_id))?,
            result,
            match (&self.validation_basis, &self.validation_warning) {
                (Some(value), None) => Some(
                    core_command_domain::orchestration::StageValidation::Basis(value.clone()),
                ),
                (None, Some(value)) => Some(
                    core_command_domain::orchestration::StageValidation::Warning(value.clone()),
                ),
                (None, None) => None,
                (Some(_), Some(_)) => return Err(DtoDecodeError::InvariantViolation),
            },
            self.source_baseline
                .as_ref()
                .map(SourceBaselineDto::to_domain)
                .transpose()?,
        )
        .map_err(|_| DtoDecodeError::InvariantViolation)
    }
}
