//! 停止判断のイベント保存専用DTO。
use super::DtoDecodeError;
use core_command_domain::orchestration::{
    ContinuationAttemptId, ContinuationRequest, ContinuationSignature, WorkflowContinuationEvent,
    WorkflowContinuationEventId, WorkflowContinuationId,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ContinuationGuardDto {
    signature: Option<String>,
    count: u64,
    initialized: bool,
}
impl ContinuationGuardDto {
    pub(super) fn of(guard: &core_command_domain::orchestration::ContinuationGuard) -> Self {
        Self {
            signature: guard.signature().map(|s| s.as_str().to_string()),
            count: guard.count(),
            initialized: guard.is_initialized(),
        }
    }
    pub(super) fn to_domain(
        &self,
    ) -> Result<core_command_domain::orchestration::ContinuationGuard, DtoDecodeError> {
        core_command_domain::orchestration::ContinuationGuard::new(
            self.signature
                .as_deref()
                .map(ContinuationSignature::parse)
                .transpose()
                .map_err(|_| DtoDecodeError::InvariantViolation)?,
            self.count,
            self.initialized,
        )
        .map_err(|_| DtoDecodeError::InvariantViolation)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ContinuationRequestDto {
    id: String,
    signature: Option<String>,
    reentrant: bool,
    limit: u64,
    wait: Option<String>,
    probe_only: bool,
    observed_guard: Option<ContinuationGuardDto>,
}
impl ContinuationRequestDto {
    pub(crate) fn of(request: &ContinuationRequest) -> Self {
        Self {
            id: request.id().to_string(),
            signature: request
                .signature()
                .map(|signature| signature.as_str().to_string()),
            reentrant: request.is_reentrant(),
            limit: request.limit(),
            wait: request.wait().map(|wait| wait.as_str().to_string()),
            probe_only: request.is_wait_probe(),
            observed_guard: request.observed_guard().map(ContinuationGuardDto::of),
        }
    }
    pub(crate) fn to_domain(&self) -> Result<ContinuationRequest, DtoDecodeError> {
        let id = ContinuationAttemptId::parse(&self.id)
            .map_err(|_| DtoDecodeError::InvariantViolation)?;
        let signature = self
            .signature
            .as_deref()
            .map(ContinuationSignature::parse)
            .transpose()
            .map_err(|_| DtoDecodeError::InvariantViolation)?;
        let wait = self
            .wait
            .as_deref()
            .map(core_command_domain::orchestration::ContinuationWait::parse)
            .transpose()
            .map_err(|_| DtoDecodeError::InvariantViolation)?;
        let request = ContinuationRequest::new(id, signature, self.reentrant, self.limit)
            .and_then(|request| request.with_wait(wait))
            .and_then(|request| {
                if self.probe_only {
                    request.with_wait_probe()
                } else {
                    Ok(request)
                }
            })
            .map_err(|_| DtoDecodeError::InvariantViolation)?;
        Ok(match &self.observed_guard {
            Some(guard) => request.with_observed_guard(guard.to_domain()?),
            None => request,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// 公開Repositoryの型引数に使う保存DTO。
#[doc(hidden)]
pub struct WorkflowContinuationEventDto {
    id: String,
    aggregate_id: String,
    request: ContinuationRequestDto,
    count: u64,
    blocked: bool,
    publication: Option<bool>,
}
impl WorkflowContinuationEventDto {
    pub(crate) fn of(event: &WorkflowContinuationEvent) -> Self {
        Self {
            id: event.id().to_string(),
            aggregate_id: event.aggregate_id().to_string(),
            request: ContinuationRequestDto::of(event.request()),
            count: event.count(),
            blocked: event.blocked(),
            publication: event.publication(),
        }
    }
    pub(crate) fn to_domain(&self) -> Result<WorkflowContinuationEvent, DtoDecodeError> {
        let id = WorkflowContinuationEventId::parse(&self.id)
            .map_err(|_| DtoDecodeError::InvariantViolation)?;
        let aggregate = WorkflowContinuationId::parse(&self.aggregate_id)
            .map_err(|_| DtoDecodeError::InvariantViolation)?;
        let event = WorkflowContinuationEvent::new(
            id,
            aggregate,
            self.request.to_domain()?,
            self.count,
            self.blocked,
        )
        .map_err(|_| DtoDecodeError::InvariantViolation)?;
        Ok(self
            .publication
            .map_or(event.clone(), |published| event.with_publication(published)))
    }
}
