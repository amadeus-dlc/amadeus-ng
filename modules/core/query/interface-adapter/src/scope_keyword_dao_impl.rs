//! `ScopeKeywordDao` の実 Gateway — キーワードから scope 名を引く。

use std::rc::Rc;

use core_query_use_case::orchestration::{ReadModelReadError, ScopeKeywordDao};

use super::read_model_store::ReadModelStore;

/// 自然キー (`definition_id`, `keyword` — UNIQUE 索引
/// `read_definition_scope_keyword_key`) の 1 行引当。
const SELECT_SCOPE_OF_KEYWORD: &str =
    "SELECT scope FROM read_definition_scope_keyword WHERE definition_id = ?1 AND keyword = ?2";

/// キーワードに割り当たった scope 名を返す実装。
#[derive(Debug)]
pub struct ScopeKeywordDaoImpl {
    store: Rc<ReadModelStore>,
}

impl ScopeKeywordDaoImpl {
    /// 1 要求ぶんの共有ストアを受け取る (**この型の唯一の構築経路**)。
    ///
    /// 開くのは [`super::ReadModelDaos`] 1 か所で、12 実装はその 1 接続を分け合う。
    /// 実装ごとに開くと、多段の引当が別々のスナップショットを見る余地が残る。
    #[must_use]
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> ScopeKeywordDaoImpl {
        ScopeKeywordDaoImpl { store }
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
