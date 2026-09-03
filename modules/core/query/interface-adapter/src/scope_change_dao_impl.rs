//! `ScopeChangeDao` の実 Gateway — 要求 scope と state の scope の照合結果を引く。

use std::path::Path;

use core_query_use_case::orchestration::{ReadModelReadError, ScopeChangeDao, ScopeChangeView};

use super::read_model_store::ReadModelStore;

/// 自然キー (`execution_id`, `scope` — UNIQUE 索引 `read_scope_change_key`) の 1 行引当。
const SELECT_SCOPE_CHANGE: &str =
    "SELECT kind FROM read_scope_change WHERE execution_id = ?1 AND scope = ?2";

/// 照合結果の綴りを返す実装。
#[derive(Debug)]
pub struct ScopeChangeDaoImpl {
    store: ReadModelStore,
}

impl ScopeChangeDaoImpl {
    /// 構造化リードモデルのストアを読取専用で開く。
    ///
    /// # Errors
    ///
    /// ストアを開けない ([`ReadModelReadError`])。
    pub fn open(path: &Path) -> Result<ScopeChangeDaoImpl, ReadModelReadError> {
        Ok(ScopeChangeDaoImpl {
            store: ReadModelStore::open(path)?,
        })
    }
}

impl ScopeChangeDao for ScopeChangeDaoImpl {
    fn find(
        &self,
        execution_id: &str,
        scope: &str,
    ) -> Result<Option<ScopeChangeView>, ReadModelReadError> {
        self.store
            .find_one(SELECT_SCOPE_CHANGE, &[&execution_id, &scope], |row| {
                Ok(ScopeChangeView::new(row.get(0)?))
            })
    }
}
