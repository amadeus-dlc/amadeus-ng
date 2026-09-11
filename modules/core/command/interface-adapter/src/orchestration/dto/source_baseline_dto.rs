//! ソース比較基準の保存形式。未採取と採取不能を区別する。
use super::dto_decode_error::DtoDecodeError;
use core_command_domain::orchestration::SourceBaseline;
use serde::{Deserialize, Serialize};
/// 外部形式の一覧バイトを所有する。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct SourceBaselineDto {
    listing: Option<String>,
}
impl SourceBaselineDto {
    pub(super) fn of(baseline: &SourceBaseline) -> Self {
        Self {
            listing: baseline.listing().map(str::to_string),
        }
    }
    pub(super) fn to_domain(&self) -> Result<SourceBaseline, DtoDecodeError> {
        SourceBaseline::new(self.listing.clone())
            .map_err(|error| DtoDecodeError::malformed("baseline", error.to_string()))
    }
}
