//! `DefinitionStageDao` の実 Gateway — グラフのステージ 1 行を引く。

use std::rc::Rc;

use core_query_use_case::orchestration::{
    DefinitionStageDao, DefinitionStageView, ReadModelReadError,
};

use super::read_model_store::ReadModelStore;

/// 自然キー (`definition_id`, `stage_slug` — UNIQUE 索引 `read_definition_stage_key`)。
const SELECT_STAGE: &str = "SELECT stage_slug, support_agents FROM read_definition_stage \
     WHERE definition_id = ?1 AND stage_slug = ?2";

/// グラフのステージ 1 行を返す実装。
#[derive(Debug)]
pub struct DefinitionStageDaoImpl {
    store: Rc<ReadModelStore>,
}

impl DefinitionStageDaoImpl {
    /// 1 要求ぶんの共有ストアを受け取る (**この型の唯一の構築経路**)。
    ///
    /// 開くのは [`super::ReadModelDaos`] 1 か所で、13 実装はその 1 接続を分け合う。
    #[must_use]
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> DefinitionStageDaoImpl {
        DefinitionStageDaoImpl { store }
    }
}

impl DefinitionStageDao for DefinitionStageDaoImpl {
    fn find(
        &self,
        definition_id: &str,
        stage_slug: &str,
    ) -> Result<Option<DefinitionStageView>, ReadModelReadError> {
        self.store
            .find_one(SELECT_STAGE, &[&definition_id, &stage_slug], |row| {
                Ok(DefinitionStageView::new(row.get(0)?, row.get(1)?))
            })
    }
}
