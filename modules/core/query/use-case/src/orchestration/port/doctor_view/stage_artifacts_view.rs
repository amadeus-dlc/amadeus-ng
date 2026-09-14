//! 配布グラフのステージが作る・消費する成果物の写し。

use super::ConsumeView;

/// `produces` / `optional_produces` / `consumes`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageArtifactsView {
    produces: Vec<String>,
    optional_produces: Vec<String>,
    consumes: Vec<ConsumeView>,
}

impl StageArtifactsView {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(
        produces: Vec<String>,
        optional_produces: Vec<String>,
        consumes: Vec<ConsumeView>,
    ) -> Self {
        Self {
            produces,
            optional_produces,
            consumes,
        }
    }

    /// 必ず作る成果物。
    #[must_use]
    pub fn produces(&self) -> &[String] {
        &self.produces
    }

    /// 作ることがある成果物。
    #[must_use]
    pub fn optional_produces(&self) -> &[String] {
        &self.optional_produces
    }

    /// 消費する成果物。
    #[must_use]
    pub fn consumes(&self) -> &[ConsumeView] {
        &self.consumes
    }
}
