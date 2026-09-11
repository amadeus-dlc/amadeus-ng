//! レビュー完了時の内容結合。
use super::{dto_decode_error::DtoDecodeError, review_binding_dto::ReviewBindingDto};
use core_command_domain::orchestration::ReviewCompletion;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ReviewCompletionDto {
    request: ReviewBindingDto,
    fingerprint: String,
}
impl ReviewCompletionDto {
    pub(super) fn of(value: &ReviewCompletion) -> Self {
        Self {
            request: ReviewBindingDto::of(value.request()),
            fingerprint: value.fingerprint().into(),
        }
    }
    pub(super) fn to_domain(&self) -> Result<ReviewCompletion, DtoDecodeError> {
        ReviewCompletion::new(self.request.to_domain()?, self.fingerprint.clone())
            .map_err(|e| DtoDecodeError::malformed("review_completion", e.to_string()))
    }
}
