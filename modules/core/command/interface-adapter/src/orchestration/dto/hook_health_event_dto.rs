//! HookHealthイベントの保存境界DTO。
use super::DtoDecodeError;
use core_command_domain::workspace::{
    HookAuditDropped, HookDropReason, HookFirstDropObserved, HookHealthEvent, HookHealthEventId,
    HookHealthId, HookHealthStarted, HookHealthTarget, HookHeartbeatObserved, HookName,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// HookHealthのイベントの永続化DTO。
pub struct HookHealthEventDto {
    id: String,
    aggregate_id: String,
    kind: String,
    target: Option<String>,
    hook: Option<String>,
    reason: Option<String>,
}
impl HookHealthEventDto {
    pub(crate) fn of(event: &HookHealthEvent) -> Self {
        let (id, aggregate_id, kind, target, hook, reason) = match event {
            HookHealthEvent::FirstDropObserved(e) => (
                e.id(),
                e.aggregate_id(),
                "first-drop",
                Some(e.target().relative_directory()),
                Some(e.hook().as_str().to_string()),
                Some(e.reason().as_str().to_string()),
            ),
            HookHealthEvent::Started(e) => (
                e.id(),
                e.aggregate_id(),
                "started",
                Some(e.target().relative_directory()),
                Some(e.hook().as_str().to_string()),
                None,
            ),
            HookHealthEvent::HeartbeatObserved(e) => {
                (e.id(), e.aggregate_id(), "heartbeat", None, None, None)
            }
            HookHealthEvent::AuditDropped(e) => (
                e.id(),
                e.aggregate_id(),
                "dropped",
                None,
                None,
                Some(e.reason().as_str().to_string()),
            ),
        };
        Self {
            id: id.to_string(),
            aggregate_id: aggregate_id.to_string(),
            kind: kind.to_string(),
            target,
            hook,
            reason,
        }
    }
    pub(crate) fn to_domain(&self) -> Result<HookHealthEvent, DtoDecodeError> {
        let id = HookHealthEventId::parse(&self.id)
            .map_err(|e| DtoDecodeError::malformed("hook_health", e.to_string()))?;
        let aid = HookHealthId::parse(&self.aggregate_id)
            .map_err(|e| DtoDecodeError::malformed("hook_health", e.to_string()))?;
        match self.kind.as_str() {
            "first-drop" => {
                Ok(HookHealthEvent::FirstDropObserved(
                    HookFirstDropObserved::new(
                        id,
                        aid,
                        HookHealthTarget::parse(self.target.as_deref().ok_or_else(|| {
                            DtoDecodeError::malformed("hook_health", "missing target")
                        })?)
                        .map_err(|e| DtoDecodeError::malformed("hook_health", e.to_string()))?,
                        HookName::parse(self.hook.as_deref().ok_or_else(|| {
                            DtoDecodeError::malformed("hook_health", "missing hook")
                        })?)
                        .map_err(|e| DtoDecodeError::malformed("hook_health", e.to_string()))?,
                        HookDropReason::parse(self.reason.as_deref().ok_or_else(|| {
                            DtoDecodeError::malformed("hook_health", "missing reason")
                        })?)
                        .map_err(|e| DtoDecodeError::malformed("hook_health", e.to_string()))?,
                    ),
                ))
            }
            "started" => {
                Ok(HookHealthEvent::Started(HookHealthStarted::new(
                    id,
                    aid,
                    HookHealthTarget::parse(self.target.as_deref().ok_or_else(|| {
                        DtoDecodeError::malformed("hook_health", "missing target")
                    })?)
                    .map_err(|e| DtoDecodeError::malformed("hook_health", e.to_string()))?,
                    HookName::parse(
                        self.hook.as_deref().ok_or_else(|| {
                            DtoDecodeError::malformed("hook_health", "missing hook")
                        })?,
                    )
                    .map_err(|e| DtoDecodeError::malformed("hook_health", e.to_string()))?,
                )))
            }
            "heartbeat" => Ok(HookHealthEvent::HeartbeatObserved(
                HookHeartbeatObserved::new(id, aid),
            )),
            "dropped" => {
                Ok(HookHealthEvent::AuditDropped(HookAuditDropped::new(
                    id,
                    aid,
                    HookDropReason::new(self.reason.clone().ok_or_else(|| {
                        DtoDecodeError::malformed("hook_health", "missing reason")
                    })?),
                )))
            }
            _ => Err(DtoDecodeError::malformed(
                "hook_health",
                "unknown hook health event",
            )),
        }
    }
}
