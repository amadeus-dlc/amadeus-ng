//! `DefinitionDao` の実 Gateway — 定義 1 行の要約を引く。

use std::rc::Rc;

use core_query_use_case::orchestration::{
    DefinitionDao, DefinitionSummaryView, ReadModelReadError,
};

use super::read_model_store::ReadModelStore;

/// 主キー `id` (= 定義の系譜 ID) の 1 行引当。
const SELECT_DEFINITION: &str =
    "SELECT revision, stage_count, scope_count FROM read_definition WHERE id = ?1";

/// 定義 1 行の要約を返す実装。
#[derive(Debug)]
pub struct DefinitionDaoImpl {
    store: Rc<ReadModelStore>,
}

impl DefinitionDaoImpl {
    /// 1 要求ぶんの共有ストアを受け取る (**この型の唯一の構築経路**)。
    ///
    /// 開くのは [`super::ReadModelDaos`] 1 か所で、12 実装はその 1 接続を分け合う。
    /// 実装ごとに開くと、多段の引当が別々のスナップショットを見る余地が残る。
    #[must_use]
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> DefinitionDaoImpl {
        DefinitionDaoImpl { store }
    }
}

impl DefinitionDao for DefinitionDaoImpl {
    fn find(
        &self,
        definition_id: &str,
    ) -> Result<Option<DefinitionSummaryView>, ReadModelReadError> {
        self.store
            .find_one(SELECT_DEFINITION, &[&definition_id], |row| {
                Ok(DefinitionSummaryView::new(
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                ))
            })
    }
}
