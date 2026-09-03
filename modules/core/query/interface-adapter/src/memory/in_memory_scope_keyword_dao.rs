//! `ScopeKeywordDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{ReadModelReadError, ScopeKeywordDao};

/// `read_definition_scope_keyword` の 1 行 (返るのは 1 列なので View 型が無い)。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    definition_id: String,
    keyword: String,
    scope: String,
}

/// キーワードに割り当たった scope 名の行を持ち、**鍵で引く**ダブル。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryScopeKeywordDao {
    held: Result<Vec<Row>, ReadModelReadError>,
}

impl InMemoryScopeKeywordDao {
    /// 握る内容をそのまま組み立てる (**この型の唯一の構造体リテラル**)。
    const fn new(held: Result<Vec<Row>, ReadModelReadError>) -> InMemoryScopeKeywordDao {
        InMemoryScopeKeywordDao { held }
    }

    /// 行が 1 つも無い (どの鍵にも当たらない)。
    #[must_use]
    pub const fn empty() -> InMemoryScopeKeywordDao {
        InMemoryScopeKeywordDao::new(Ok(Vec::new()))
    }

    /// 行を 1 つ足す (鍵は定義 × キーワード)。
    #[must_use]
    pub fn with_row(
        mut self,
        definition_id: &str,
        keyword: &str,
        scope: &str,
    ) -> InMemoryScopeKeywordDao {
        if let Ok(rows) = &mut self.held {
            rows.push(Row {
                definition_id: definition_id.to_string(),
                keyword: keyword.to_string(),
                scope: scope.to_string(),
            });
        }
        self
    }

    /// 引けない。
    #[must_use]
    pub const fn failing(error: ReadModelReadError) -> InMemoryScopeKeywordDao {
        InMemoryScopeKeywordDao::new(Err(error))
    }
}

impl ScopeKeywordDao for InMemoryScopeKeywordDao {
    fn find(
        &self,
        definition_id: &str,
        keyword: &str,
    ) -> Result<Option<String>, ReadModelReadError> {
        let rows = self.held.as_ref().map_err(Clone::clone)?;
        Ok(rows
            .iter()
            .find(|row| row.definition_id == definition_id && row.keyword == keyword)
            .map(|row| row.scope.clone()))
    }
}
