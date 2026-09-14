//! `WorkspaceDoctorEvent` の保存境界 DTO。
use super::{DtoDecodeError, WorkspaceDoctorCheckDto};
use core_command_domain::workspace::{
    DoctorCheck, DoctorChecks, HookHealthTarget, WorkspaceDoctorEvent, WorkspaceDoctorEventId,
    WorkspaceDoctorId,
};
use core_infrastructure::collections::FirstClassCollection as _;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// 1 回の診断事実の永続化 DTO。行は集約の判断の写しなので、そのまま運ぶ。
pub struct WorkspaceDoctorEventDto {
    id: String,
    aggregate_id: String,
    target: String,
    checks: Vec<WorkspaceDoctorCheckDto>,
}
impl WorkspaceDoctorEventDto {
    pub(crate) fn of(event: &WorkspaceDoctorEvent) -> Self {
        Self {
            id: event.id().to_string(),
            aggregate_id: event.aggregate_id().to_string(),
            target: event.target().relative_directory(),
            checks: event.checks().fold_left(Vec::new(), |mut rows, check| {
                rows.push(WorkspaceDoctorCheckDto::of(check));
                rows
            }),
        }
    }
    pub(crate) fn to_domain(&self) -> Result<WorkspaceDoctorEvent, DtoDecodeError> {
        let id = WorkspaceDoctorEventId::parse(&self.id)
            .map_err(|e| DtoDecodeError::malformed("workspace_doctor", e.to_string()))?;
        let aggregate_id = WorkspaceDoctorId::parse(&self.aggregate_id)
            .map_err(|e| DtoDecodeError::malformed("workspace_doctor", e.to_string()))?;
        let target = HookHealthTarget::parse(&self.target)
            .map_err(|e| DtoDecodeError::malformed("workspace_doctor", e.to_string()))?;
        let mut checks: Vec<DoctorCheck> = Vec::with_capacity(self.checks.len());
        for row in &self.checks {
            checks.push(row.to_domain()?);
        }
        WorkspaceDoctorEvent::new(id, aggregate_id, target, DoctorChecks::new(checks))
            .map_err(|e| DtoDecodeError::malformed("workspace_doctor", e.to_string()))
    }
}
