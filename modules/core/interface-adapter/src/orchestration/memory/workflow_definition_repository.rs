//! `WorkflowDefinitionRepository` の in-memory 実装 (テスト用)。
//!
//! 3 入力を固定値として与え、ファイル I/O 抜きで述語 6 種のユースケーステストを回すための
//! Gateway (12-workflow-definition §9-3)。集約は構築後 immutable なので、`find_by_id` は
//! 保持している `WorkflowDefinition` の複製を返す。識別子の契約 (要求 id が保持している定義の
//! id と違えば `NotFound`) は実 Gateway と同じに保つ。
//!
//! テストダブルなので `Impl` 接尾辞は付けない (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md)。

use core_domain::workflow_definition::{WorkflowDefinition, WorkflowDefinitionId};
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
    fn find_by_id(&self, id: &WorkflowDefinitionId) -> Result<WorkflowDefinition, GraphReadError> {
        // テストダブルでも識別子の契約は本物と同じ — 「1 Repository 1 定義、要求 id が
        // 違えば NotFound」(BR2.6)。違うのは 3 入力のパースと失敗注入が無いことだけ。
        if self.definition.id() != id {
            return Err(GraphReadError::NotFound {
                expected: self.definition.id().clone(),
                actual: id.clone(),
            });
        }
        Ok(self.definition.clone())
    }
}

#[cfg(test)]
mod tests {
    // panic! は想定外バリアントの即時失敗という検証用途で使っており、テスト失敗の
    // シグナルとして妥当なため許容する。
    #![allow(clippy::panic)]

    use super::*;
    use core_domain::workflow_definition::{
        DefinitionRevision, ExecutionKind, PhaseId, ScopeGrid, ScopeMetadata, StageGraph,
        StageMode, StageNodeBuilder, StageNumber, StageSlug,
    };
    use std::collections::BTreeMap;

    fn id(value: &str) -> WorkflowDefinitionId {
        WorkflowDefinitionId::parse(value).unwrap()
    }

    fn revision() -> DefinitionRevision {
        DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap()
    }

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
        let grid = ScopeGrid::from_graph(&graph);
        let mut scopes = BTreeMap::new();
        scopes.insert(
            "feature".to_string(),
            ScopeMetadata::new("feature").unwrap(),
        );
        WorkflowDefinition::new(id("claude"), revision(), graph, grid, scopes)
    }

    #[test]
    fn find_by_id_returns_the_seeded_definition_every_time() {
        let reader = InMemoryWorkflowDefinitionRepository::new(definition());
        let first = reader.find_by_id(&id("claude")).unwrap();
        let second = reader.find_by_id(&id("claude")).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.valid_scopes(), ["feature"]);
        assert_eq!(reader.definition(), &first);
    }

    #[test]
    fn find_by_id_carries_the_identity_and_the_revision_of_the_seeded_definition() {
        let reader = InMemoryWorkflowDefinitionRepository::new(definition());
        let found = reader.find_by_id(&id("claude")).unwrap();
        assert_eq!(found.id(), &id("claude"));
        assert_eq!(found.revision(), &revision());
    }

    #[test]
    fn a_request_for_another_definition_is_not_found() {
        // テストダブルも実 Gateway と同じ契約を守る (12 §9-3)。
        let reader = InMemoryWorkflowDefinitionRepository::new(definition());
        assert_eq!(
            reader.find_by_id(&id("kiro")),
            Err(GraphReadError::NotFound {
                expected: id("claude"),
                actual: id("kiro"),
            })
        );
    }

    #[test]
    fn the_not_found_error_names_the_provider_as_expected_and_the_request_as_actual() {
        let reader = InMemoryWorkflowDefinitionRepository::new(definition());
        let error = reader.find_by_id(&id("codex")).unwrap_err();
        let GraphReadError::NotFound { expected, actual } = error else {
            panic!("NotFound を期待した");
        };
        // expected = この Repository が提供できる id、actual = 要求された id。
        assert_eq!(expected.as_str(), "claude");
        assert_eq!(actual.as_str(), "codex");
    }

    #[test]
    fn the_seeded_definition_is_never_mutated_by_a_rejected_request() {
        let reader = InMemoryWorkflowDefinitionRepository::new(definition());
        let before = reader.definition().clone();
        assert!(reader.find_by_id(&id("kiro")).is_err());
        assert_eq!(reader.definition(), &before);
    }
}
