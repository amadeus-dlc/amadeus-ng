//! `PhaseEntryDao` の実 Gateway — 定義側のフェーズ入口を引く。

use std::rc::Rc;

use core_query_use_case::orchestration::{PhaseEntryDao, PhaseEntryView, ReadModelReadError};

use super::read_model_store::ReadModelStore;

/// 自然キー (`definition_id`, `scope`, `phase` — UNIQUE 索引
/// `read_definition_scope_phase_entry_key`) の 1 行引当。
const SELECT_PHASE_ENTRY: &str = "SELECT first_stage_slug FROM read_definition_scope_phase_entry \
WHERE definition_id = ?1 AND scope = ?2 AND phase = ?3";

/// 定義側のフェーズ入口を返す実装。
#[derive(Debug)]
pub struct PhaseEntryDaoImpl {
    store: Rc<ReadModelStore>,
}

impl PhaseEntryDaoImpl {
    /// 1 要求ぶんの共有ストアを受け取る (**この型の唯一の構築経路**)。
    ///
    /// 開くのは [`super::ReadModelDaos`] 1 か所で、12 実装はその 1 接続を分け合う。
    /// 実装ごとに開くと、多段の引当が別々のスナップショットを見る余地が残る。
    #[must_use]
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> PhaseEntryDaoImpl {
        PhaseEntryDaoImpl { store }
    }
}

impl PhaseEntryDao for PhaseEntryDaoImpl {
    fn find(
        &self,
        definition_id: &str,
        scope: &str,
        phase: &str,
    ) -> Result<Option<PhaseEntryView>, ReadModelReadError> {
        self.store.find_one(
            SELECT_PHASE_ENTRY,
            &[&definition_id, &scope, &phase],
            |row| Ok(PhaseEntryView::new(row.get(0)?)),
        )
    }
}
