//! `JumpPhaseDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{JumpPhaseDao, JumpPhaseView, ReadModelReadError};

/// `read_next_jump_phase` の 1 行 (自然キー 2 列は View が運ばないので明示的に持つ)。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    execution_id: String,
    phase: String,
    view: JumpPhaseView,
}

/// フェーズの目的地の行を持ち、**鍵で引く**ダブル。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryJumpPhaseDao {
    held: Result<Vec<Row>, ReadModelReadError>,
}

impl InMemoryJumpPhaseDao {
    /// 握る内容をそのまま組み立てる (**この型の唯一の構造体リテラル**)。
    const fn new(held: Result<Vec<Row>, ReadModelReadError>) -> InMemoryJumpPhaseDao {
        InMemoryJumpPhaseDao { held }
    }

    /// 行が 1 つも無い (どの鍵にも当たらない)。
    #[must_use]
    pub const fn empty() -> InMemoryJumpPhaseDao {
        InMemoryJumpPhaseDao::new(Ok(Vec::new()))
    }

    /// 行を 1 つ足す (鍵は `execution_id` × `phase`)。
    #[must_use]
    pub fn with_row(
        mut self,
        execution_id: &str,
        phase: &str,
        view: JumpPhaseView,
    ) -> InMemoryJumpPhaseDao {
        if let Ok(rows) = &mut self.held {
            rows.push(Row {
                execution_id: execution_id.to_string(),
                phase: phase.to_string(),
                view,
            });
        }
        self
    }

    /// 引けない。
    #[must_use]
    pub const fn failing(error: ReadModelReadError) -> InMemoryJumpPhaseDao {
        InMemoryJumpPhaseDao::new(Err(error))
    }
}

impl JumpPhaseDao for InMemoryJumpPhaseDao {
    fn find(
        &self,
        execution_id: &str,
        phase: &str,
    ) -> Result<Option<JumpPhaseView>, ReadModelReadError> {
        let rows = self.held.as_ref().map_err(Clone::clone)?;
        Ok(rows
            .iter()
            .find(|row| row.execution_id == execution_id && row.phase == phase)
            .map(|row| row.view.clone()))
    }
}
