//! `WorkspaceDoctor` スナップショットのワイヤ DTO。
use super::{DtoDecodeError, WorkspaceDoctorCheckDto};
use chrono::{DateTime, Utc};
use core_command_domain::workspace::{
    DoctorCheck, DoctorChecks, HookHealthTarget, WorkspaceDoctor, WorkspaceDoctorId,
};
use core_infrastructure::collections::FirstClassCollection as _;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// 診断集約のスナップショットの永続化 DTO。
pub struct WorkspaceDoctorDto {
    id: String,
    target: String,
    checks: Vec<WorkspaceDoctorCheckDto>,
    seq_nr: usize,
    version: usize,
    diagnosed_at: DateTime<Utc>,
}
impl WorkspaceDoctorDto {
    pub(crate) fn of(value: &WorkspaceDoctor) -> Self {
        Self {
            id: value.id().to_string(),
            target: value.target().relative_directory(),
            checks: value.checks().fold_left(Vec::new(), |mut rows, check| {
                rows.push(WorkspaceDoctorCheckDto::of(check));
                rows
            }),
            seq_nr: value.seq_nr(),
            version: value.version(),
            diagnosed_at: value.diagnosed_at(),
        }
    }
    pub(crate) fn to_domain(&self) -> Result<WorkspaceDoctor, DtoDecodeError> {
        let id = WorkspaceDoctorId::parse(&self.id)
            .map_err(|e| DtoDecodeError::malformed("workspace_doctor", e.to_string()))?;
        let target = HookHealthTarget::parse(&self.target)
            .map_err(|e| DtoDecodeError::malformed("workspace_doctor", e.to_string()))?;
        let mut checks: Vec<DoctorCheck> = Vec::with_capacity(self.checks.len());
        for row in &self.checks {
            checks.push(row.to_domain()?);
        }
        WorkspaceDoctor::new(
            id,
            target,
            DoctorChecks::new(checks),
            self.seq_nr,
            self.version,
            self.diagnosed_at,
        )
        .map_err(|e| DtoDecodeError::malformed("workspace_doctor", e.to_string()))
    }
}
