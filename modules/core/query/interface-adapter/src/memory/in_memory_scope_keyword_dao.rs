//! `ScopeKeywordDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{ReadModelReadError, ScopeKeywordDao};

/// キーワード引当の結果を握るダブル (鍵は見ない)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryScopeKeywordDao {
    held: Result<Option<String>, ReadModelReadError>,
}

impl InMemoryScopeKeywordDao {
    /// その語に scope が割り当たっている。
    #[must_use]
    pub const fn holding(scope: String) -> InMemoryScopeKeywordDao {
        InMemoryScopeKeywordDao {
            held: Ok(Some(scope)),
        }
    }

    /// どの scope のキーワードでもない。
    #[must_use]
    pub const fn absent() -> InMemoryScopeKeywordDao {
        InMemoryScopeKeywordDao { held: Ok(None) }
    }

    /// 引けない。
    #[must_use]
    pub const fn failing(error: ReadModelReadError) -> InMemoryScopeKeywordDao {
        InMemoryScopeKeywordDao { held: Err(error) }
    }
}

impl ScopeKeywordDao for InMemoryScopeKeywordDao {
    fn find(&self, _: &str, _: &str) -> Result<Option<String>, ReadModelReadError> {
        self.held.clone()
    }
}
