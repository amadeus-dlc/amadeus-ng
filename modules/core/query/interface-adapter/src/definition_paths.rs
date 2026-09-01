//! `DefinitionPaths` — 3 入力の置き場。パス解決とテストシーム。

use std::path::PathBuf;

/// グラフ成果物のファイル名 (`<harnessRoot>/tools/data/` 直下)。
const STAGE_GRAPH_FILE: &str = "stage-graph.json";
/// グリッド成果物のファイル名 (同上)。
const SCOPE_GRID_FILE: &str = "scope-grid.json";
/// ハーネス identity ファイルの名前 (同上)。定義 id の供給元 (ADR-008)。
const HARNESS_FILE: &str = "harness.json";

/// 3 入力の置き場 — パス解決とテストシーム。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionPaths {
    data_dir: PathBuf,
    pub(super) scopes_dir: PathBuf,
    pub(super) stage_graph_override: Option<PathBuf>,
    scope_grid_override: Option<PathBuf>,
}

impl DefinitionPaths {
    /// `data_dir` は `stage-graph.json` / `scope-grid.json` / `harness.json` の置き場
    /// (`<harnessRoot>/tools/data/`)、`scopes_dir` は identity ファイルの置き場
    /// (`<harnessRoot>/scopes/`)。
    #[must_use]
    pub const fn new(data_dir: PathBuf, scopes_dir: PathBuf) -> DefinitionPaths {
        DefinitionPaths {
            data_dir,
            scopes_dir,
            stage_graph_override: None,
            scope_grid_override: None,
        }
    }

    /// `AIDLC_STAGE_GRAPH` 相当のオーバライド。設定すると読取失敗時の逐語文言の hint 節が
    /// 「unset して既定に戻せ」形へ切り替わる (12 §4 #1)。
    #[must_use]
    pub fn with_stage_graph_override(mut self, path: PathBuf) -> DefinitionPaths {
        self.stage_graph_override = Some(path);
        self
    }

    /// `AIDLC_SCOPE_GRID` 相当のオーバライド。グリッドの欠損は fatal ではないため、
    /// こちらに hint 節の分岐は無い。
    #[must_use]
    pub fn with_scope_grid_override(mut self, path: PathBuf) -> DefinitionPaths {
        self.scope_grid_override = Some(path);
        self
    }

    /// 解決済みの `stage-graph.json` パス。
    #[must_use]
    pub fn stage_graph_path(&self) -> PathBuf {
        self.stage_graph_override
            .clone()
            .unwrap_or_else(|| self.data_dir.join(STAGE_GRAPH_FILE))
    }

    /// 解決済みの `scope-grid.json` パス。
    #[must_use]
    pub fn scope_grid_path(&self) -> PathBuf {
        self.scope_grid_override
            .clone()
            .unwrap_or_else(|| self.data_dir.join(SCOPE_GRID_FILE))
    }

    /// 解決済みの `harness.json` パス。env オーバライドは無い (upstream に対応する env が
    /// 無く、identity はハーネスの配置そのものだからである)。
    #[must_use]
    pub fn harness_path(&self) -> PathBuf {
        self.data_dir.join(HARNESS_FILE)
    }
}
