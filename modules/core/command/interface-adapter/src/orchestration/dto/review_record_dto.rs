//! 現試行のレビュー結合履歴。
use super::{
    dto_decode_error::DtoDecodeError, review_binding_dto::ReviewBindingDto,
    review_completion_dto::ReviewCompletionDto,
};
use core_command_domain::orchestration::ReviewRecord;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum ReviewRecordDto {
    Requested {
        iteration: u32,
        binding: ReviewBindingDto,
        retry: bool,
    },
    Completed {
        iteration: u32,
        completion: ReviewCompletionDto,
    },
}
impl ReviewRecordDto {
    pub(super) fn of(value: &ReviewRecord) -> Self {
        match value {
            ReviewRecord::Requested {
                iteration,
                binding,
                retry,
            } => Self::Requested {
                iteration: *iteration,
                binding: ReviewBindingDto::of(binding),
                retry: *retry,
            },
            ReviewRecord::Completed {
                iteration,
                completion,
            } => Self::Completed {
                iteration: *iteration,
                completion: ReviewCompletionDto::of(completion),
            },
        }
    }
    pub(super) fn to_domain(&self) -> Result<ReviewRecord, DtoDecodeError> {
        Ok(match self {
            Self::Requested {
                iteration,
                binding,
                retry,
            } => ReviewRecord::Requested {
                iteration: *iteration,
                binding: binding.to_domain()?,
                retry: *retry,
            },
            Self::Completed {
                iteration,
                completion,
            } => ReviewRecord::Completed {
                iteration: *iteration,
                completion: completion.to_domain()?,
            },
        })
    }
}
