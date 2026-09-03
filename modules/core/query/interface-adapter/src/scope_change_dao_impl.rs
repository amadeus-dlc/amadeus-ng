//! `ScopeChangeDao` の実 Gateway — 要求 scope と state の scope の照合結果を引く。

use std::rc::Rc;

use core_query_use_case::orchestration::{ReadModelReadError, ScopeChangeDao, ScopeChangeView};

use super::read_model_store::ReadModelStore;

/// 自然キー (`execution_id`, `scope` — UNIQUE 索引 `read_scope_change_key`) の 1 行引当。
const SELECT_SCOPE_CHANGE: &str =
    "SELECT kind FROM read_scope_change WHERE execution_id = ?1 AND scope = ?2";

/// 照合結果の綴りを返す実装。
#[derive(Debug)]
pub struct ScopeChangeDaoImpl {
    store: Rc<ReadModelStore>,
}

impl ScopeChangeDaoImpl {
    /// 1 要求ぶんの共有ストアを受け取る (**この型の唯一の構築経路**)。
    ///
    /// 開くのは [`super::ReadModelDaos`] 1 か所で、12 実装はその 1 接続を分け合う。
    /// 実装ごとに開くと、多段の引当が別々のスナップショットを見る余地が残る。
    #[must_use]
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> ScopeChangeDaoImpl {
        ScopeChangeDaoImpl { store }
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
