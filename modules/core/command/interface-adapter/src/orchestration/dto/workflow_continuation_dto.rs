//! 停止制御集約の保存専用DTO。
use super::DtoDecodeError;
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{WorkflowContinuation, WorkflowContinuationId};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// 公開Repositoryの型引数に使う保存DTO。
#[doc(hidden)]
pub struct WorkflowContinuationDto {
    id: String,
    request: super::workflow_continuation_event_dto::ContinuationRequestDto,
    before: super::workflow_continuation_event_dto::ContinuationGuardDto,
    selected: super::workflow_continuation_event_dto::ContinuationGuardDto,
    published: Option<bool>,
    seq_nr: usize,
    version: usize,
    occurred_at: DateTime<Utc>,
}
impl WorkflowContinuationDto {
    pub(crate) fn of(aggregate: &WorkflowContinuation) -> Self {
        Self {
            id: aggregate.id().to_string(),
            request: super::workflow_continuation_event_dto::ContinuationRequestDto::of(
                aggregate.last_request(),
            ),
            before: super::workflow_continuation_event_dto::ContinuationGuardDto::of(
                aggregate.counter().before(),
            ),
            selected: super::workflow_continuation_event_dto::ContinuationGuardDto::of(
                aggregate.counter().selected(),
            ),
            published: aggregate.counter().published(),
            seq_nr: aggregate.seq_nr(),
            version: aggregate.version(),
            occurred_at: aggregate.occurred_at(),
        }
    }
    pub(crate) fn to_domain(&self) -> Result<WorkflowContinuation, DtoDecodeError> {
        let id = WorkflowContinuationId::parse(&self.id)
            .map_err(|_| DtoDecodeError::InvariantViolation)?;
        WorkflowContinuation::new(
            id,
            self.request.to_domain()?,
            core_command_domain::orchestration::ContinuationCounter::new(
                self.before.to_domain()?,
                self.selected.to_domain()?,
                self.published,
            ),
            self.seq_nr,
            self.version,
            self.occurred_at,
        )
        .map_err(|_| DtoDecodeError::InvariantViolation)
    }
}
