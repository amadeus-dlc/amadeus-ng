//! HookHealth保存境界のワイヤDTO。
use super::DtoDecodeError;
use chrono::{DateTime, Utc};
use core_command_domain::workspace::{
    HookDropReason, HookDropSummary, HookHealth, HookHealthId, HookHealthTarget, HookName,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// HookHealthの集約スナップショットの永続化DTO。
pub struct HookHealthDto {
    id: String,
    target: String,
    hook: String,
    heartbeat: Option<DateTime<Utc>>,
    observed_at: DateTime<Utc>,
    seq_nr: usize,
    version: usize,
    drops: usize,
    latest_drop: Option<String>,
}
impl HookHealthDto {
    pub(crate) fn of(value: &HookHealth) -> Self {
        Self {
            id: value.id().to_string(),
            target: value.target().relative_directory(),
            hook: value.hook().as_str().to_string(),
            heartbeat: value.heartbeat(),
            observed_at: value.observed_at(),
            seq_nr: value.seq_nr(),
            version: value.version(),
            drops: value.drops(),
            latest_drop: value.latest_drop().map(|r| r.as_str().to_string()),
        }
    }
    pub(crate) fn to_domain(&self) -> Result<HookHealth, DtoDecodeError> {
        let id = HookHealthId::parse(&self.id)
            .map_err(|e| DtoDecodeError::malformed("hook_health", e.to_string()))?;
        let target = HookHealthTarget::parse(&self.target)
            .map_err(|e| DtoDecodeError::malformed("hook_health", e.to_string()))?;
        let hook = HookName::parse(&self.hook)
            .map_err(|e| DtoDecodeError::malformed("hook_health", e.to_string()))?;
        let drop = self.latest_drop.clone().map(HookDropReason::new);
        let drop = HookDropSummary::new(self.drops, drop)
            .map_err(|e| DtoDecodeError::malformed("hook_health", e.to_string()))?;
        HookHealth::new(
            id,
            target,
            hook,
            self.heartbeat,
            self.observed_at,
            self.seq_nr,
            self.version,
            drop,
        )
        .map_err(|e| DtoDecodeError::malformed("hook_health", e.to_string()))
    }
}
