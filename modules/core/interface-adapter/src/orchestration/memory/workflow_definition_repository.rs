//! `WorkflowDefinitionRepository` の in-memory 実装 (テスト用)。
//!
//! 3 入力を固定値として与え、ファイル I/O 抜きで述語 5 種のユースケーステストを回すための
//! Gateway (12-workflow-definition §9-3)。集約は構築後 immutable なので、`load` は
//! 保持している `WorkflowDefinition` の複製をそのまま返す。
//!
//! テストダブルなので `Impl` 接尾辞は付けない (docs/memory/gateway-taxonomy.md)。

use core_domain::workflow_definition::WorkflowDefinition;
use core_use_case::orchestration::{GraphReadError, WorkflowDefinitionRepository};

/// 組み立て済みの `WorkflowDefinition` を保持するだけの `WorkflowDefinitionRepository`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryWorkflowDefinitionRepository {
    definition: WorkflowDefinition,
}

impl InMemoryWorkflowDefinitionRepository {
    /// 組み立て済みの読取モデルを固定値として据える。`find` はこれを複製して返すだけで、
    /// 3 入力のパースも失敗注入も行わない (失敗態度の検証は `workflow_definition_repository_impl` 側)。
    #[must_use]
    pub const fn new(definition: WorkflowDefinition) -> InMemoryWorkflowDefinitionRepository {
        InMemoryWorkflowDefinitionRepository { definition }
    }

    /// 保持している読取モデルへの参照 (テストの組み立て確認用)。
    #[must_use]
    pub const fn definition(&self) -> &WorkflowDefinition {
        &self.definition
    }
}

impl WorkflowDefinitionRepository for InMemoryWorkflowDefinitionRepository {
    fn find(&self) -> Result<WorkflowDefinition, GraphReadError> {
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
        let reader = InMemoryWorkflowDefinitionRepository::new(definition());
        let first = reader.find().unwrap();
        let second = reader.find().unwrap();
        assert_eq!(first, second);
        assert_eq!(first.valid_scopes(), ["feature"]);
        assert_eq!(reader.definition(), &first);
    }
}
