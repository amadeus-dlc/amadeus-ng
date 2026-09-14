//! 配布グラフ (`stage-graph.json`) のステージ 1 件の写し — 診断が見る列だけ。

use super::StageArtifactsView;

/// slug・phase・番号・有効フラグ・依存と成果物。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphStageView {
    slug: String,
    phase: String,
    number: String,
    enabled: bool,
    requires_stage: Vec<String>,
    artifacts: StageArtifactsView,
}

impl GraphStageView {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(
        slug: String,
        phase: String,
        number: String,
        enabled: bool,
        requires_stage: Vec<String>,
        artifacts: StageArtifactsView,
    ) -> Self {
        Self {
            slug,
            phase,
            number,
            enabled,
            requires_stage,
            artifacts,
        }
    }

    /// ステージ slug。
    #[must_use]
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// フェーズ名。
    #[must_use]
    pub fn phase(&self) -> &str {
        &self.phase
    }

    /// 表示番号 (`2.7` 等)。
    #[must_use]
    pub fn number(&self) -> &str {
        &self.number
    }

    /// `enabled !== false` か。
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// 先行ステージの slug。
    #[must_use]
    pub fn requires_stage(&self) -> &[String] {
        &self.requires_stage
    }

    /// 成果物 (作る・消費する)。
    #[must_use]
    pub const fn artifacts(&self) -> &StageArtifactsView {
        &self.artifacts
    }
}
