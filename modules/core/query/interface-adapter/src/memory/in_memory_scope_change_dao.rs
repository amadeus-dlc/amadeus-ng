//! `ScopeChangeDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{ReadModelReadError, ScopeChangeDao, ScopeChangeView};

/// `read_scope_change` の 1 行 (自然キー 2 列は View が運ばない)。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    execution_id: String,
    scope: String,
    view: ScopeChangeView,
}

/// scope 照合結果の行を持ち、**鍵で引く**ダブル。
///
/// **行が無いこと自体が「無効な scope」の答え**なので、鍵を見ないダブルではこの答えを
/// 表せない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryScopeChangeDao {
    held: Result<Vec<Row>, ReadModelReadError>,
}

impl InMemoryScopeChangeDao {
    /// 握る内容をそのまま組み立てる (**この型の唯一の構造体リテラル**)。
    const fn new(held: Result<Vec<Row>, ReadModelReadError>) -> InMemoryScopeChangeDao {
        InMemoryScopeChangeDao { held }
    }

    /// 行が 1 つも無い (どの鍵にも当たらない)。
    #[must_use]
    pub const fn empty() -> InMemoryScopeChangeDao {
        InMemoryScopeChangeDao::new(Ok(Vec::new()))
    }

    /// 行を 1 つ足す (鍵は実行 × 要求 scope)。
    #[must_use]
    pub fn with_row(
        mut self,
        execution_id: &str,
        scope: &str,
        view: ScopeChangeView,
    ) -> InMemoryScopeChangeDao {
        if let Ok(rows) = &mut self.held {
            rows.push(Row {
                execution_id: execution_id.to_string(),
                scope: scope.to_string(),
                view,
            });
        }
        self
    }

    /// 引けない。
    #[must_use]
    pub const fn failing(error: ReadModelReadError) -> InMemoryScopeChangeDao {
        InMemoryScopeChangeDao::new(Err(error))
    }
}

impl ScopeChangeDao for InMemoryScopeChangeDao {
    fn find(
        &self,
        execution_id: &str,
        scope: &str,
    ) -> Result<Option<ScopeChangeView>, ReadModelReadError> {
        let rows = self.held.as_ref().map_err(Clone::clone)?;
        Ok(rows
            .iter()
            .find(|row| row.execution_id == execution_id && row.scope == scope)
            .map(|row| row.view.clone()))
    }
}
