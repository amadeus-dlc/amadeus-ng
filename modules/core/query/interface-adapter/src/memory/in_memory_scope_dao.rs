//! `ScopeDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{ReadModelReadError, ScopeDao, ScopeView};

/// `read_definition_scope` の 1 行 (`definition_id` は View が運ばないので明示的に持つ)。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    definition_id: String,
    view: ScopeView,
}

/// scope カタログの行を持ち、**鍵で引く**ダブル。
///
/// 既製 3 scope の引当 ([`ScopeDao::find_stock`]) は trait の既定実装が
/// [`ScopeDao::find`] を定数の順に 3 回呼ぶ形なので、鍵で引き分けられることがそのまま
/// 並び順の契約になる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryScopeDao {
    held: Result<Vec<Row>, ReadModelReadError>,
}

impl InMemoryScopeDao {
    /// 握る内容をそのまま組み立てる (**この型の唯一の構造体リテラル**)。
    const fn new(held: Result<Vec<Row>, ReadModelReadError>) -> InMemoryScopeDao {
        InMemoryScopeDao { held }
    }

    /// 行が 1 つも無い (どの鍵にも当たらない)。
    #[must_use]
    pub const fn empty() -> InMemoryScopeDao {
        InMemoryScopeDao::new(Ok(Vec::new()))
    }

    /// 行を 1 つ足す (鍵は `definition_id` と行の `scope`)。
    #[must_use]
    pub fn with_row(mut self, definition_id: &str, view: ScopeView) -> InMemoryScopeDao {
        if let Ok(rows) = &mut self.held {
            rows.push(Row {
                definition_id: definition_id.to_string(),
                view,
            });
        }
        self
    }

    /// 引けない。
    #[must_use]
    pub const fn failing(error: ReadModelReadError) -> InMemoryScopeDao {
        InMemoryScopeDao::new(Err(error))
    }
}

impl ScopeDao for InMemoryScopeDao {
    fn find(
        &self,
        definition_id: &str,
        scope: &str,
    ) -> Result<Option<ScopeView>, ReadModelReadError> {
        let rows = self.held.as_ref().map_err(Clone::clone)?;
        Ok(rows
            .iter()
            .find(|row| row.definition_id == definition_id && row.view.scope() == scope)
            .map(|row| row.view.clone()))
    }
}
