//! jump観測の保存形式。
use super::{dto_decode_error::DtoDecodeError, source_baseline_dto::SourceBaselineDto};
use core_command_domain::orchestration::{JumpArtifact, JumpObservation};
use core_command_domain::workflow_definition::StageSlug;
use serde::{Deserialize, Serialize};
/// ソース一覧と成果物観測の独立DTO。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct JumpObservationDto {
    baseline: SourceBaselineDto,
    artifacts: Vec<ArtifactDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ArtifactDto {
    stage: String,
    path: String,
    exists: bool,
    review: bool,
}
impl JumpObservationDto {
    pub(super) fn of(value: &JumpObservation) -> Self {
        Self {
            baseline: SourceBaselineDto::of(value.baseline()),
            artifacts: value.fold_artifacts(Vec::new(), |mut rows, artifact| {
                rows.push(ArtifactDto {
                    stage: artifact.stage().as_str().into(),
                    path: artifact.path().into(),
                    exists: artifact.exists(),
                    review: artifact.has_review(),
                });
                rows
            }),
        }
    }
    pub(super) fn to_domain(&self) -> Result<JumpObservation, DtoDecodeError> {
        let artifacts = self
            .artifacts
            .iter()
            .map(|a| {
                Ok(JumpArtifact::new(
                    StageSlug::parse(&a.stage)
                        .map_err(|_| DtoDecodeError::malformed("stage", &a.stage))?,
                    a.path.clone(),
                    a.exists,
                    a.review,
                ))
            })
            .collect::<Result<Vec<_>, DtoDecodeError>>()?;
        Ok(JumpObservation::new(self.baseline.to_domain()?, artifacts))
    }
}
