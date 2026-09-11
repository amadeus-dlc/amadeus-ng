//! 引継ぎファイルに関する境界観測。読取成否を業務の判定と混同しない。
use super::PipelineHandoff;
/// 指定パスと、期待する場所のファイル観測。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineHandoffInput {
    supplied: Option<String>,
    expected: String,
    matches_path: bool,
    observation: Result<Option<PipelineHandoff>, String>,
}
impl PipelineHandoffInput {
    /// 指定値とI/Oの観測を完全に束ねる。
    #[must_use]
    pub const fn new(
        supplied: Option<String>,
        expected: String,
        matches_path: bool,
        observation: Result<Option<PipelineHandoff>, String>,
    ) -> Self {
        Self {
            supplied,
            expected,
            matches_path,
            observation,
        }
    }
    pub(super) fn current(&self) -> Option<&PipelineHandoff> {
        self.observation.as_ref().ok().and_then(Option::as_ref)
    }
    pub(super) fn require(&self) -> Result<PipelineHandoff, super::PipelineLinkError> {
        use super::PipelineLinkError as E;
        let supplied = self
            .supplied
            .as_ref()
            .filter(|s| !s.is_empty())
            .ok_or(E::ArtifactRequired)?;
        if !self.matches_path {
            return Err(E::ArtifactPath {
                expected: self.expected.clone(),
            });
        }
        match &self.observation {
            Ok(Some(snapshot)) => Ok(snapshot.clone()),
            Ok(None) => Err(E::ArtifactMissing {
                supplied: supplied.clone(),
            }),
            Err(cause) => Err(E::ArtifactUnreadable {
                cause: cause.clone(),
            }),
        }
    }
}
