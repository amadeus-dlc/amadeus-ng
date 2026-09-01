//! `WorkflowDefinitionDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{
    DefinitionView, WorkflowDefinitionDao, WorkflowDefinitionReadError,
};

/// 定義リードモデルの読取結果を握るダブル。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryWorkflowDefinitionDao {
    held: Result<DefinitionView, WorkflowDefinitionReadError>,
}

impl InMemoryWorkflowDefinitionDao {
    /// 読める定義。
    #[must_use]
    pub const fn holding(view: DefinitionView) -> InMemoryWorkflowDefinitionDao {
        InMemoryWorkflowDefinitionDao { held: Ok(view) }
    }

    /// 読取に失敗する定義 (読取対象を名指しできない `Unidentified` を含む)。
    #[must_use]
    pub const fn failing(error: WorkflowDefinitionReadError) -> InMemoryWorkflowDefinitionDao {
        InMemoryWorkflowDefinitionDao { held: Err(error) }
    }
}

impl WorkflowDefinitionDao for InMemoryWorkflowDefinitionDao {
    fn find(&self) -> Result<DefinitionView, WorkflowDefinitionReadError> {
        self.held.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_query_use_case::orchestration::{
        DefinitionIdView, DefinitionRevisionView, ExecutionKindView, PhaseView, ScopeGridView,
        StageGraphView, StageModeView, StageNumberView, StageSlugView, StageViewBuilder,
    };
    use std::collections::BTreeMap;

    fn view() -> DefinitionView {
        DefinitionView::new(
            DefinitionIdView::parse("claude").unwrap(),
            DefinitionRevisionView::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            StageGraphView::new(vec![
                StageViewBuilder::new(
                    StageSlugView::parse("stage-0").unwrap(),
                    StageNumberView::parse("0.1").unwrap(),
                    "Stage 0".to_string(),
                    PhaseView::Initialization,
                    ExecutionKindView::Always,
                    StageModeView::Inline,
                )
                .with_lead_agent("orchestrator".to_string())
                .with_scopes(vec!["classic".to_string()])
                .build(),
            ])
            .unwrap(),
            ScopeGridView::new(BTreeMap::new()),
            BTreeMap::new(),
        )
    }

    #[test]
    fn the_double_replays_whatever_it_was_given() {
        assert_eq!(
            InMemoryWorkflowDefinitionDao::holding(view())
                .find()
                .unwrap(),
            view()
        );
        assert_eq!(
            InMemoryWorkflowDefinitionDao::failing(WorkflowDefinitionReadError::Unidentified)
                .find()
                .unwrap_err(),
            WorkflowDefinitionReadError::Unidentified
        );
    }

    #[test]
    fn reading_twice_yields_the_same_answer() {
        let workflow_definition_dao = InMemoryWorkflowDefinitionDao::holding(view());
        assert_eq!(
            workflow_definition_dao.find().unwrap(),
            workflow_definition_dao.find().unwrap()
        );
    }
}
