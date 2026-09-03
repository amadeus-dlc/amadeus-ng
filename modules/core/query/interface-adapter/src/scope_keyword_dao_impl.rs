//! `ScopeKeywordDao` の実 Gateway — キーワードから scope 名を引く。

use std::path::Path;

use core_query_use_case::orchestration::{ReadModelReadError, ScopeKeywordDao};

use super::read_model_store::ReadModelStore;

/// 自然キー (`definition_id`, `keyword` — UNIQUE 索引
/// `read_definition_scope_keyword_key`) の 1 行引当。
const SELECT_SCOPE_OF_KEYWORD: &str =
    "SELECT scope FROM read_definition_scope_keyword WHERE definition_id = ?1 AND keyword = ?2";

/// キーワードに割り当たった scope 名を返す実装。
#[derive(Debug)]
pub struct ScopeKeywordDaoImpl {
    store: ReadModelStore,
}

impl ScopeKeywordDaoImpl {
    /// 構造化リードモデルのストアを読取専用で開く。
    ///
    /// # Errors
    ///
    /// ストアを開けない ([`ReadModelReadError`])。
    pub fn open(path: &Path) -> Result<ScopeKeywordDaoImpl, ReadModelReadError> {
        Ok(ScopeKeywordDaoImpl {
            store: ReadModelStore::open(path)?,
        })
    }
}

impl ScopeKeywordDao for ScopeKeywordDaoImpl {
    fn find(
        &self,
        definition_id: &str,
        keyword: &str,
    ) -> Result<Option<String>, ReadModelReadError> {
        self.store.find_one(
            SELECT_SCOPE_OF_KEYWORD,
            &[&definition_id, &keyword],
            |row| row.get(0),
        )
    }
}
