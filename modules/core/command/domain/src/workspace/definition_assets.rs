//! D3.b〜D3.d の観測 — 配布グラフ・スコープ表・スコープ定義・ステージ本体・エージェント。

use super::{GraphStage, ObservationFailure, ScopeGridEntry, StageFile};

/// 配布資産の読取結果。各資産は独立に「読めた / 読めなかった」を運ぶ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionAssets {
    graph: Result<Vec<GraphStage>, ObservationFailure>,
    scope_grid: Result<Vec<ScopeGridEntry>, ObservationFailure>,
    scope_names: Result<Vec<String>, ObservationFailure>,
    stage_files: Result<Vec<StageFile>, ObservationFailure>,
    agents: Result<Vec<String>, ObservationFailure>,
}

impl DefinitionAssets {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(
        graph: Result<Vec<GraphStage>, ObservationFailure>,
        scope_grid: Result<Vec<ScopeGridEntry>, ObservationFailure>,
        scope_names: Result<Vec<String>, ObservationFailure>,
        stage_files: Result<Vec<StageFile>, ObservationFailure>,
        agents: Result<Vec<String>, ObservationFailure>,
    ) -> Self {
        Self {
            graph,
            scope_grid,
            scope_names,
            stage_files,
            agents,
        }
    }

    /// `stage-graph.json` の全ステージ (無効なものも含む)。
    pub const fn graph(&self) -> &Result<Vec<GraphStage>, ObservationFailure> {
        &self.graph
    }

    /// `scope-grid.json` の全スコープ。
    pub const fn scope_grid(&self) -> &Result<Vec<ScopeGridEntry>, ObservationFailure> {
        &self.scope_grid
    }

    /// `.claude/scopes/*.md` の frontmatter `name` (名前順)。
    pub const fn scope_names(&self) -> &Result<Vec<String>, ObservationFailure> {
        &self.scope_names
    }

    /// `.claude/aidlc-common/stages/<phase>/*.md` の全ファイル (フェーズ順・名前順)。
    pub const fn stage_files(&self) -> &Result<Vec<StageFile>, ObservationFailure> {
        &self.stage_files
    }

    /// `.claude/agents/*.md` の frontmatter `name` (名前順)。
    pub const fn agents(&self) -> &Result<Vec<String>, ObservationFailure> {
        &self.agents
    }
}
