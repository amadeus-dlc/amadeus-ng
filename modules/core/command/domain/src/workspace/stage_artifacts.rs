//! 配布グラフのステージが作る・消費する成果物の写し。

use super::ConsumedArtifact;

/// `produces` / `optional_produces` / `consumes`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageArtifacts {
    produces: Vec<String>,
    optional_produces: Vec<String>,
    consumes: Vec<ConsumedArtifact>,
}

impl StageArtifacts {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(
        produces: Vec<String>,
        optional_produces: Vec<String>,
        consumes: Vec<ConsumedArtifact>,
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
    pub fn consumes(&self) -> &[ConsumedArtifact] {
        &self.consumes
    }

    /// この行が作る成果物名 (必須と任意の両方)。
    pub fn produced_names(&self) -> impl Iterator<Item = &str> {
        self.produces
            .iter()
            .chain(&self.optional_produces)
            .map(String::as_str)
    }
}
