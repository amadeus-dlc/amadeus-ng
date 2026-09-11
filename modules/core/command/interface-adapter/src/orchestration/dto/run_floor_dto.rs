//! 実行境界のスナップショット表現。
use super::dto_decode_error::DtoDecodeError;
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{CodeGenerationRunFloor, RunBoundaryKind};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct RunFloorDto {
    workflow_started: u64,
    stage_started: u64,
    stage_jumped: u64,
    gate_rejected: u64,
    latest: Option<BoundaryDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct BoundaryDto {
    kind: String,
    at: DateTime<Utc>,
}
impl RunFloorDto {
    pub(super) fn of(floor: &CodeGenerationRunFloor) -> Self {
        Self {
            workflow_started: floor.count(RunBoundaryKind::WorkflowStarted),
            stage_started: floor.count(RunBoundaryKind::StageStarted),
            stage_jumped: floor.count(RunBoundaryKind::StageJumped),
            gate_rejected: floor.count(RunBoundaryKind::GateRejected),
            latest: floor.latest().map(|(kind, at)| BoundaryDto {
                kind: kind.as_str().to_string(),
                at,
            }),
        }
    }
    pub(super) fn to_domain(&self) -> Result<CodeGenerationRunFloor, DtoDecodeError> {
        let latest = self
            .latest
            .as_ref()
            .map(|boundary| {
                let kind = match boundary.kind.as_str() {
                    "WORKFLOW_STARTED" => RunBoundaryKind::WorkflowStarted,
                    "STAGE_STARTED" => RunBoundaryKind::StageStarted,
                    "STAGE_JUMPED" => RunBoundaryKind::StageJumped,
                    "GATE_REJECTED" => RunBoundaryKind::GateRejected,
                    _ => return Err(DtoDecodeError::malformed("run_floor.kind", &boundary.kind)),
                };
                Ok((kind, boundary.at))
            })
            .transpose()?;
        CodeGenerationRunFloor::new(
            self.workflow_started,
            self.stage_started,
            self.stage_jumped,
            self.gate_rejected,
            latest,
        )
        .map_err(|error| DtoDecodeError::malformed("run_floor", error.to_string()))
    }
}
