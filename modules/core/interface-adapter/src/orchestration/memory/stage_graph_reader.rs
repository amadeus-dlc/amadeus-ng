//! `StageGraphReader` の in-memory 実装 (テスト用)。
//!
//! 3 入力を固定値として与え、ファイル I/O 抜きで述語 5 種のユースケーステストを回すための
//! Gateway (12-workflow-definition §9-3)。読取モデルは構築後 immutable なので、`load` は
//! 保持している `WorkflowDefinition` の複製をそのまま返す。

use core_domain::workflow_definition::WorkflowDefinition;
use core_use_case::orchestration::{GraphReadError, StageGraphReader};

/// 組み立て済みの `WorkflowDefinition` を保持するだけの `StageGraphReader`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryStageGraphReader {
    definition: WorkflowDefinition,
}

impl InMemoryStageGraphReader {
    /// 組み立て済みの読取モデルを固定値として据える。`load` はこれを複製して返すだけで、
    /// 3 入力のパースも失敗注入も行わない (失敗態度の検証は `fs_stage_graph_reader` 側)。
    #[must_use]
    pub const fn new(definition: WorkflowDefinition) -> InMemoryStageGraphReader {
        InMemoryStageGraphReader { definition }
    }

    /// 保持している読取モデルへの参照 (テストの組み立て確認用)。
    #[must_use]
    pub const fn definition(&self) -> &WorkflowDefinition {
        &self.definition
    }
}

impl StageGraphReader for InMemoryStageGraphReader {
    fn load(&self) -> Result<WorkflowDefinition, GraphReadError> {
        Ok(self.definition.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_domain::workflow_definition::{
        ExecutionKind, PhaseId, ScopeGrid, ScopeMetadata, StageGraph, StageMode, StageNodeBuilder,
        StageNumber, StageSlug,
    };
    use std::collections::BTreeMap;

    fn definition() -> WorkflowDefinition {
        let node = StageNodeBuilder::new(
            StageSlug::parse("intent-capture").unwrap(),
            StageNumber::parse("1.1").unwrap(),
            "Intent Capture".to_string(),
            PhaseId::Ideation,
            ExecutionKind::Always,
            StageMode::Inline,
        )
        .scopes(vec!["feature".to_string()])
        .build();
        let graph = StageGraph::new(vec![node]).unwrap();
        let grid = ScopeGrid::derive_from_graph(&graph);
        let mut scopes = BTreeMap::new();
        scopes.insert(
            "feature".to_string(),
            ScopeMetadata::new("feature").unwrap(),
        );
        WorkflowDefinition::new(graph, grid, scopes)
    }

    #[test]
    fn load_returns_the_seeded_definition_every_time() {
        let reader = InMemoryStageGraphReader::new(definition());
        let first = reader.load().unwrap();
        let second = reader.load().unwrap();
        assert_eq!(first, second);
        assert_eq!(first.valid_scopes(), ["feature"]);
        assert_eq!(reader.definition(), &first);
    }
}
