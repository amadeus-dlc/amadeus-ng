//! `JumpPhaseDao` の実 Gateway — フェーズの目的地 1 行を `read_next_jump_phase` から引く。

use std::rc::Rc;

use core_query_use_case::orchestration::{JumpPhaseDao, JumpPhaseView, ReadModelReadError};

use super::read_model_store::ReadModelStore;

/// 自然キー (`execution_id`, `phase` — UNIQUE 索引 `read_next_jump_phase_key`) の 1 行引当。
///
/// 受理判定 (`read_next_jump`) は結合しない — 行が言うのは目的地の位置までで、そこから
/// たどるのはユースケースの仕事である (オーナー裁定 2026-09-03 —
/// `coding-rules/cqrs-boundaries.md` 規則 6)。
const SELECT_PHASE_TARGET: &str = "SELECT target_index, target_slug \
FROM read_next_jump_phase WHERE execution_id = ?1 AND phase = ?2";

/// フェーズの目的地 1 行を返す実装。
#[derive(Debug)]
pub struct JumpPhaseDaoImpl {
    store: Rc<ReadModelStore>,
}

impl JumpPhaseDaoImpl {
    /// 1 要求ぶんの共有ストアを受け取る (**この型の唯一の構築経路**)。
    ///
    /// 開くのは [`super::ReadModelDaos`] 1 か所で、12 実装はその 1 接続を分け合う。
    /// 実装ごとに開くと、多段の引当が別々のスナップショットを見る余地が残る。
    #[must_use]
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> JumpPhaseDaoImpl {
        JumpPhaseDaoImpl { store }
    }
}

impl JumpPhaseDao for JumpPhaseDaoImpl {
    fn find(
        &self,
        execution_id: &str,
        phase: &str,
    ) -> Result<Option<JumpPhaseView>, ReadModelReadError> {
        self.store
            .find_one(SELECT_PHASE_TARGET, &[&execution_id, &phase], |row| {
                Ok(JumpPhaseView::new(row.get(0)?, row.get(1)?))
            })
    }
}
