//! `ScopeDao` の実 Gateway — scope カタログ 1 列を引く。

use std::path::Path;

use core_query_use_case::orchestration::{ReadModelReadError, ScopeDao, ScopeView};

use super::read_model_store::ReadModelStore;

/// 自然キー (`definition_id`, `scope` — UNIQUE 索引 `read_definition_scope_key`) の 1 行引当。
const SELECT_SCOPE: &str = "SELECT scope, depth, keywords, skeleton, review_cap, \
freeform_default, has_grid_column, cost_total, cost_execute, cost_gates, cost_per_unit_stages \
FROM read_definition_scope WHERE definition_id = ?1 AND scope = ?2";

/// scope カタログ 1 列を返す実装。
///
/// 既製 3 scope は trait の既定実装が [`ScopeDao::find`] を定数の順に 3 回呼ぶ — 並び順を
/// SQL の `CASE` に持たせると upstream の定数が 2 か所に散るので、鍵の並びはポートに置いた
/// ままにする。
#[derive(Debug)]
pub struct ScopeDaoImpl {
    store: ReadModelStore,
}

impl ScopeDaoImpl {
    /// 構造化リードモデルのストアを読取専用で開く。
    ///
    /// # Errors
    ///
    /// ストアを開けない ([`ReadModelReadError`])。
    pub fn open(path: &Path) -> Result<ScopeDaoImpl, ReadModelReadError> {
        Ok(ScopeDaoImpl {
            store: ReadModelStore::open(path)?,
        })
    }
}

impl ScopeDao for ScopeDaoImpl {
    fn find(
        &self,
        definition_id: &str,
        scope: &str,
    ) -> Result<Option<ScopeView>, ReadModelReadError> {
        self.store
            .find_one(SELECT_SCOPE, &[&definition_id, &scope], |row| {
                Ok(ScopeView::new(
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                ))
            })
    }
}
