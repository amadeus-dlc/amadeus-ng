//! `ExecutionStateDao` の in-memory テストダブル。

use core_query_use_case::execution_view::ExecutionStateView;
use core_query_use_case::orchestration::{ExecutionStateDao, ExecutionStateReadError};

/// 実行状態リードモデルの読取結果を握るダブル。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryExecutionStateDao {
    held: Result<Option<ExecutionStateView>, ExecutionStateReadError>,
}

impl InMemoryExecutionStateDao {
    /// 稼働中のリードモデルがある。
    #[must_use]
    pub const fn holding(view: ExecutionStateView) -> InMemoryExecutionStateDao {
        InMemoryExecutionStateDao {
            held: Ok(Some(view)),
        }
    }

    /// リードモデルが無い (誕生分岐へ — 正常な観測であって失敗ではない)。
    #[must_use]
    pub const fn absent() -> InMemoryExecutionStateDao {
        InMemoryExecutionStateDao { held: Ok(None) }
    }

    /// 在るのに読めない。
    #[must_use]
    pub const fn failing(error: ExecutionStateReadError) -> InMemoryExecutionStateDao {
        InMemoryExecutionStateDao { held: Err(error) }
    }
}

impl ExecutionStateDao for InMemoryExecutionStateDao {
    fn find(&self) -> Result<Option<ExecutionStateView>, ExecutionStateReadError> {
        self.held.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_query_use_case::execution_view::{CheckboxState, ExecutionStatus, StageProgressView};
    use core_query_use_case::workflow_view::{
        PhaseView, PlanActionView, ScopeSlugView, StageSlugView,
    };

    fn view() -> ExecutionStateView {
        ExecutionStateView::new(
            ScopeSlugView::parse("classic").unwrap(),
            ExecutionStatus::Running,
            "stage-0",
            None,
            "2026-08-29T16:36:24Z",
            vec![StageProgressView::new(
                StageSlugView::parse("stage-0").unwrap(),
                PhaseView::Initialization,
                CheckboxState::InProgress,
                PlanActionView::Execute,
            )],
        )
        .unwrap()
    }

    #[test]
    fn the_double_replays_whatever_it_was_given() {
        assert_eq!(
            InMemoryExecutionStateDao::holding(view()).find().unwrap(),
            Some(view())
        );
        assert_eq!(InMemoryExecutionStateDao::absent().find().unwrap(), None);
        let error = ExecutionStateReadError::NotReadable {
            path: "/r/aidlc-state.md".to_string(),
            cause: "permission denied".to_string(),
        };
        assert_eq!(
            InMemoryExecutionStateDao::failing(error.clone())
                .find()
                .unwrap_err(),
            error
        );
    }

    #[test]
    fn reading_twice_yields_the_same_answer() {
        let dao = InMemoryExecutionStateDao::holding(view());
        assert_eq!(dao.find().unwrap(), dao.find().unwrap());
    }
}
