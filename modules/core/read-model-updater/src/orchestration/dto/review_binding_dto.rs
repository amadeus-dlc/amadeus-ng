//! レビュー要求の内容結合。書込み側とRMUがそれぞれ所有する。
use super::dto_decode_error::DtoDecodeError;
use core_command_domain::orchestration::ReviewBinding;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ReviewBindingDto {
    fingerprint: String,
    appendix_artifact: String,
    appendix_offset: usize,
    prior_digest: String,
    prior_length: usize,
    challenge: Option<String>,
    source: Option<String>,
}
impl ReviewBindingDto {
    pub(super) fn of(value: &ReviewBinding) -> Self {
        Self {
            fingerprint: value.fingerprint().into(),
            appendix_artifact: value.appendix_artifact().into(),
            appendix_offset: value.appendix_offset(),
            prior_digest: value.prior_digest().into(),
            prior_length: value.prior_length(),
            challenge: value.challenge().map(str::to_string),
            source: value.source().map(str::to_string),
        }
    }
    pub(super) fn to_domain(&self) -> Result<ReviewBinding, DtoDecodeError> {
        ReviewBinding::new(
            self.fingerprint.clone(),
            self.appendix_artifact.clone(),
            self.appendix_offset,
            self.prior_digest.clone(),
            self.prior_length,
            self.challenge.clone(),
            self.source.clone(),
        )
        .map_err(|e| DtoDecodeError::malformed("review_binding", e.to_string()))
    }
}
