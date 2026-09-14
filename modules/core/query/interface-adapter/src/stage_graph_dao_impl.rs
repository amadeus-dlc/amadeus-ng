//! `StageGraphDao` の実 Gateway — `stage-graph.json` を読んでステージ 1 ノード / 全ノードを写す。
//!
//! 媒体は JSON ファイルであり、SQLite の `read_*` 表とは別の面である — したがって
//! [`super::ReadModelDaos`] (1 要求 1 接続) の住人ではなく、配布データ置き場だけを握る。
//! `stage-graph.json` は compile コンテキストの投影 (リードモデル) であり、それを読む・パース
//! する実装がクエリ側に在るのは規則どおりである (`coding-rules/cqrs-boundaries.md` 規則 7)。

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use core_query_use_case::orchestration::{ReadModelReadError, StageGraphDao, StageGraphEntryView};

/// コンパイル済みステージグラフを 1 面読む実装。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageGraphDaoImpl {
    stage_graph: PathBuf,
}

impl StageGraphDaoImpl {
    /// 配布データ置き場 (`<harness>/tools/data`) を受け取る (**この型の唯一の構築経路**)。
    #[must_use]
    pub fn new(definition_data_dir: &Path) -> StageGraphDaoImpl {
        StageGraphDaoImpl {
            stage_graph: definition_data_dir.join("stage-graph.json"),
        }
    }

    /// 有効な全ノードをグラフ順で読む。`enabled: false` のノードは除く
    /// (upstream `loadStageGraph` — `s.enabled !== false`)。
    fn load_enabled(&self) -> Result<Vec<StageGraphEntryView>, ReadModelReadError> {
        // 状態ファイルと違い、グラフは在って当然のリードモデルなので不在も読取失敗として扱う
        // (upstream `loadStageGraph` は不在で throw する)。
        let raw = std::fs::read_to_string(&self.stage_graph).map_err(|error| {
            ReadModelReadError::new(error.kind(), Some(self.stage_graph.clone()))
        })?;
        let parsed: serde_json::Value = serde_json::from_str(&raw).map_err(|_| {
            ReadModelReadError::new(ErrorKind::InvalidData, Some(self.stage_graph.clone()))
        })?;
        let nodes = parsed.as_array().ok_or_else(|| {
            ReadModelReadError::new(ErrorKind::InvalidData, Some(self.stage_graph.clone()))
        })?;
        let mut out = Vec::new();
        for node in nodes {
            if node.get("enabled").and_then(serde_json::Value::as_bool) == Some(false) {
                continue;
            }
            out.push(view_of(node));
        }
        Ok(out)
    }
}

/// 1 ノードの JSON から使う 8 列を写す (欠けた文字列列は空、`support_agents` は空配列)。
fn view_of(node: &serde_json::Value) -> StageGraphEntryView {
    StageGraphEntryView::new(
        text(node, "slug"),
        text(node, "number"),
        text(node, "name"),
        text(node, "phase"),
        text(node, "execution"),
        text(node, "lead_agent"),
        strings(node.get("support_agents")),
        text(node, "mode"),
    )
}

fn text(node: &serde_json::Value, key: &str) -> String {
    node.get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn strings(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect()
}

impl StageGraphDao for StageGraphDaoImpl {
    fn find(
        &self,
        slug_or_number: &str,
    ) -> Result<Option<StageGraphEntryView>, ReadModelReadError> {
        let all = self.load_enabled()?;
        // upstream `resolveStage` = slug で引き、無ければ番号で引く。
        Ok(all
            .iter()
            .find(|stage| stage.slug() == slug_or_number)
            .or_else(|| all.iter().find(|stage| stage.number() == slug_or_number))
            .cloned())
    }

    fn find_all(&self) -> Result<Vec<StageGraphEntryView>, ReadModelReadError> {
        self.load_enabled()
    }
}
