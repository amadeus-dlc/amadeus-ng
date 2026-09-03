//! `RunStageDao` の実 Gateway — run-stage の材料 1 行を `read_run_stage` から引く。

use std::rc::Rc;

use core_query_use_case::orchestration::{ReadModelReadError, RunStageDao, RunStageView};

use super::read_model_store::ReadModelStore;
use super::run_stage_columns::{run_stage_row, select_run_stage};

/// 自然キー (`definition_id`, `scope`, `stage_slug` — UNIQUE 索引 `read_run_stage_key`)。
const SELECT_BY_NATURAL_KEY: &str =
    select_run_stage!("definition_id = ?1 AND scope = ?2 AND stage_slug = ?3");

/// 主キー (`read_next_answer.run_stage_id` がこの値を指す)。
const SELECT_BY_ID: &str = select_run_stage!("id = ?1");

/// 自然キー + token の 2 束縛。束縛は照合ではなく**鍵の残余条件**である。
const SELECT_BOUND: &str = select_run_stage!(
    "definition_id = ?1 AND scope = ?2 AND stage_slug = ?3 \
     AND route_digest = ?4 AND directive_digest = ?5"
);

/// run-stage の材料 1 行を返す実装 (3 動詞とも同じ 1 表を鍵違いで引く)。
#[derive(Debug)]
pub struct RunStageDaoImpl {
    store: Rc<ReadModelStore>,
}

impl RunStageDaoImpl {
    /// 1 要求ぶんの共有ストアを受け取る (**この型の唯一の構築経路**)。
    ///
    /// 開くのは [`super::ReadModelDaos`] 1 か所で、12 実装はその 1 接続を分け合う。
    /// 実装ごとに開くと、多段の引当が別々のスナップショットを見る余地が残る。
    #[must_use]
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> RunStageDaoImpl {
        RunStageDaoImpl { store }
    }
}

impl RunStageDao for RunStageDaoImpl {
    fn find(
        &self,
        definition_id: &str,
        scope: &str,
        stage_slug: &str,
    ) -> Result<Option<RunStageView>, ReadModelReadError> {
        self.store.find_one(
            SELECT_BY_NATURAL_KEY,
            &[&definition_id, &scope, &stage_slug],
            run_stage_row,
        )
    }

    fn find_by_id(&self, id: &str) -> Result<Option<RunStageView>, ReadModelReadError> {
        self.store.find_one(SELECT_BY_ID, &[&id], run_stage_row)
    }

    fn find_bound(
        &self,
        definition_id: &str,
        scope: &str,
        stage_slug: &str,
        route_digest: &str,
        directive_digest: &str,
    ) -> Result<Option<RunStageView>, ReadModelReadError> {
        self.store.find_one(
            SELECT_BOUND,
            &[
                &definition_id,
                &scope,
                &stage_slug,
                &route_digest,
                &directive_digest,
            ],
            run_stage_row,
        )
    }
}
