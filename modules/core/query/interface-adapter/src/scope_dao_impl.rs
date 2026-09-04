//! `ScopeDao` の実 Gateway — scope カタログ 1 列を引く。

use std::rc::Rc;

use core_query_use_case::orchestration::{ReadModelReadError, ScopeDao, ScopeView};

use super::read_model_store::ReadModelStore;

/// 自然キー (`definition_id`, `scope` — UNIQUE 索引 `read_definition_scope_key`) の 1 行引当。
const SELECT_SCOPE: &str = "SELECT scope, depth, keywords, skeleton, review_cap, \
freeform_default, has_grid_column, cost_total, cost_execute, cost_gates, cost_per_unit_stages \
FROM read_definition_scope WHERE definition_id = ?1 AND scope = ?2";

/// 定義 1 本の scope 列を綴り順で引く (索引列 `definition_id` の残余なし引当)。
///
/// 引く表は [`SELECT_SCOPE`] と同じ 1 表である。`ORDER BY` を置くのは、行が位置の列を
/// 持たないため並びを決めるものが他に無いからであり、選別ではない。
const SELECT_SCOPES: &str = "SELECT scope, depth, keywords, skeleton, review_cap, \
freeform_default, has_grid_column, cost_total, cost_execute, cost_gates, cost_per_unit_stages \
FROM read_definition_scope WHERE definition_id = ?1 ORDER BY scope";

/// scope カタログ 1 列を返す実装。
///
/// 既製 3 scope は trait の既定実装が [`ScopeDao::find`] を定数の順に 3 回呼ぶ — 並び順を
/// SQL の `CASE` に持たせると upstream の定数が 2 か所に散るので、鍵の並びはポートに置いた
/// ままにする。
#[derive(Debug)]
pub struct ScopeDaoImpl {
    store: Rc<ReadModelStore>,
}

impl ScopeDaoImpl {
    /// 1 要求ぶんの共有ストアを受け取る (**この型の唯一の構築経路**)。
    ///
    /// 開くのは [`super::ReadModelDaos`] 1 か所で、12 実装はその 1 接続を分け合う。
    /// 実装ごとに開くと、多段の引当が別々のスナップショットを見る余地が残る。
    #[must_use]
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> ScopeDaoImpl {
        ScopeDaoImpl { store }
    }
}

/// 1 行を [`ScopeView`] へ写す (2 つの引当が同じ列の並びを読む)。
fn view_of(row: &rusqlite::Row<'_>) -> rusqlite::Result<ScopeView> {
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
}

impl ScopeDao for ScopeDaoImpl {
    fn find_all(&self, definition_id: &str) -> Result<Vec<ScopeView>, ReadModelReadError> {
        self.store
            .find_many(SELECT_SCOPES, &[&definition_id], view_of)
    }

    fn find(
        &self,
        definition_id: &str,
        scope: &str,
    ) -> Result<Option<ScopeView>, ReadModelReadError> {
        self.store
            .find_one(SELECT_SCOPE, &[&definition_id, &scope], view_of)
    }
}
