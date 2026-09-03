//! `ExecutionDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{ExecutionDao, ExecutionView, ReadModelReadError};

/// 実行の現在地の行を持ち、**鍵で引く**ダブル。
///
/// 2 つの鍵 (`id` / `state_binding`) はどちらも行が運ぶ列なので、明示的な鍵引数を取らず
/// View から読む — 鍵と行の値がずれる余地を作らない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryExecutionDao {
    held: Result<Vec<ExecutionView>, ReadModelReadError>,
}

impl InMemoryExecutionDao {
    /// 握る内容をそのまま組み立てる (**この型の唯一の構造体リテラル**)。
    const fn new(held: Result<Vec<ExecutionView>, ReadModelReadError>) -> InMemoryExecutionDao {
        InMemoryExecutionDao { held }
    }

    /// 行が 1 つも無い (どの鍵にも当たらない)。
    #[must_use]
    pub const fn empty() -> InMemoryExecutionDao {
        InMemoryExecutionDao::new(Ok(Vec::new()))
    }

    /// 行を 1 つ足す (鍵はどちらも行の列)。
    #[must_use]
    pub fn with_row(mut self, view: ExecutionView) -> InMemoryExecutionDao {
        if let Ok(rows) = &mut self.held {
            rows.push(view);
        }
        self
    }

    /// 引けない。
    #[must_use]
    pub const fn failing(error: ReadModelReadError) -> InMemoryExecutionDao {
        InMemoryExecutionDao::new(Err(error))
    }
}

impl ExecutionDao for InMemoryExecutionDao {
    fn find(&self, execution_id: &str) -> Result<Option<ExecutionView>, ReadModelReadError> {
        let rows = self.held.as_ref().map_err(Clone::clone)?;
        Ok(rows
            .iter()
            .find(|view| view.execution_id() == execution_id)
            .cloned())
    }

    fn find_by_state_binding(
        &self,
        state_binding: &str,
    ) -> Result<Option<ExecutionView>, ReadModelReadError> {
        let rows = self.held.as_ref().map_err(Clone::clone)?;
        Ok(rows
            .iter()
            .find(|view| view.state_binding() == state_binding)
            .cloned())
    }
}
