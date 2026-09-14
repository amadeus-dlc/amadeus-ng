//! 配布グラフ (`stage-graph.json`) のステージ 1 件の写し — 診断が見る列だけ。

use super::StageArtifacts;

/// slug・phase・番号・有効フラグ・依存と成果物。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphStage {
    slug: String,
    phase: String,
    number: String,
    enabled: bool,
    requires_stage: Vec<String>,
    artifacts: StageArtifacts,
}

impl GraphStage {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(
        slug: String,
        phase: String,
        number: String,
        enabled: bool,
        requires_stage: Vec<String>,
        artifacts: StageArtifacts,
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
    pub const fn artifacts(&self) -> &StageArtifacts {
        &self.artifacts
    }

    /// 本家 `numericStageOrder` — `<phase>.<index>` を整数対で比べる。
    #[must_use]
    pub fn numeric_order(&self, other: &Self) -> std::cmp::Ordering {
        let parse = |number: &str| -> (i64, i64) {
            let mut parts = number.split('.');
            let phase = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
            let index = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
            (phase, index)
        };
        parse(&self.number).cmp(&parse(&other.number))
    }

    /// 自分自身を先行ステージに名指しているか。
    #[must_use]
    pub fn requires_itself(&self) -> bool {
        self.requires_stage.iter().any(|dep| dep == &self.slug)
    }
}
