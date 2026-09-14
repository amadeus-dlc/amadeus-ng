//! 自己診断の 1 行の保存境界 DTO — ワイヤ形式はこの層が決める。
use super::DtoDecodeError;
use core_command_domain::workspace::{DoctorCheck, DoctorCheckId};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// C7 の 1 行 (検査 ID・成否・ラベル・修復案) の永続化 DTO。
pub struct WorkspaceDoctorCheckDto {
    check_id: String,
    passed: bool,
    label: String,
    fix: Option<String>,
}
impl WorkspaceDoctorCheckDto {
    pub(crate) fn of(check: &DoctorCheck) -> Self {
        Self {
            check_id: check.id().as_str().to_string(),
            passed: check.is_passed(),
            label: check.label().to_string(),
            fix: check.fix().map(str::to_string),
        }
    }
    pub(crate) fn to_domain(&self) -> Result<DoctorCheck, DtoDecodeError> {
        let id = DoctorCheckId::parse(&self.check_id)
            .map_err(|e| DtoDecodeError::malformed("workspace_doctor", e.to_string()))?;
        Ok(DoctorCheck::new(
            id,
            self.passed,
            self.label.clone(),
            self.fix.clone(),
        ))
    }
}
