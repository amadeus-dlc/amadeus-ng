//! `RuntimeGraphTargets` — runtime-graph の投影が読む面と書く面。

use std::path::{Path, PathBuf};

/// runtime-graph の投影先と、その材料になる記録面の場所。
///
/// 記録ディレクトリ 1 本から導く — 別 intent の監査シャードと別 intent の
/// `runtime-graph.json` を取り合わせられないようにするためである
/// ([`ProjectionTargets`] と同じ理由)。
///
/// 出力は `<record>/runtime-graph.json` である (固定本家 2.7.1 の `runtimeGraphPath`)。
/// 再生成可能な派生物なので配布の `.gitignore` が
/// `aidlc/spaces/*/intents/*/runtime-graph.json` として除外している。
///
/// [`ProjectionTargets`]: super::ProjectionTargets
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeGraphTargets {
    audit_dir: PathBuf,
    state_file: PathBuf,
    graph_file: PathBuf,
    record_prefix: String,
}

impl RuntimeGraphTargets {
    /// 記録ディレクトリと、その**プロジェクト相対**の綴りから組む (**唯一の構築経路**)。
    ///
    /// `record_prefix` は行の `memory_path` に前置する綴りであり、固定本家の
    /// `relativeRecordDir` に対応する (`aidlc/spaces/<space>/intents/<record>`)。
    #[must_use]
    pub fn new(record_dir: impl Into<PathBuf>, record_prefix: impl Into<String>) -> Self {
        let record_dir = record_dir.into();
        Self {
            audit_dir: record_dir.join("audit"),
            state_file: record_dir.join("aidlc-state.md"),
            graph_file: record_dir.join("runtime-graph.json"),
            record_prefix: record_prefix.into(),
        }
    }

    /// 監査シャードの置き場。
    #[must_use]
    pub fn audit_dir(&self) -> &Path {
        &self.audit_dir
    }

    /// 実行状態リードモデル (`Scope` の正本)。
    #[must_use]
    pub fn state_file(&self) -> &Path {
        &self.state_file
    }

    /// runtime-graph の投影先。
    #[must_use]
    pub fn graph_file(&self) -> &Path {
        &self.graph_file
    }

    /// 行の `memory_path` に前置する記録の綴り。
    #[must_use]
    pub fn record_prefix(&self) -> &str {
        &self.record_prefix
    }
}

#[cfg(test)]
mod tests {
    use super::RuntimeGraphTargets;
    use std::path::Path;

    #[test]
    fn the_targets_are_derived_from_one_record_directory() {
        let targets = RuntimeGraphTargets::new(
            Path::new("/w/aidlc/spaces/default/intents/260909-demo"),
            "aidlc/spaces/default/intents/260909-demo",
        );
        assert!(targets.audit_dir().ends_with("260909-demo/audit"));
        assert!(targets.state_file().ends_with("260909-demo/aidlc-state.md"));
        assert!(
            targets
                .graph_file()
                .ends_with("260909-demo/runtime-graph.json")
        );
        assert_eq!(
            targets.record_prefix(),
            "aidlc/spaces/default/intents/260909-demo"
        );
    }
}
